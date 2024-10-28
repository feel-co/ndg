{
  lib,
  # build dependencies
  runCommandLocal,
  pandoc,
  nixosOptionsDoc,
  ndg-stylesheet,
  # options
  genJsonDocs ? true,
  checkModules ? false,
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
      modules = rawModules ++ [{_module.check = checkModules;}];
      inherit specialArgs;
    },
  title ? "My Option Documentation",
  templatePath ? ./assets/default-template.html,
  styleSheetPath ? ./assets/default-styles.scss,
  codeThemePath ? ./assets/default-syntax.theme,
  optionsDocArgs ? {},
  sandboxing ? true,
  embedResources ? false,
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

  configJSON =
    (nixosOptionsDoc (
      (removeAttrs optionsDocArgs ["options"])
      // {inherit (evaluatedModules) options;}
    ))
    .optionsJSON;
in
  runCommandLocal "generate-option-docs.html" {nativeBuildInputs = [pandoc];} (
    ''
      mkdir -p $out/share/doc

      ${optionalString genJsonDocs ''
        cp -vf ${configJSON}/share/doc/nixos/options.json $out/share/doc/options.json
      ''}

      # Convert to Pandoc markdown instead of using commonmark directly
      # as the former automatically generates heading IDs and TOC links.
      pandoc \
        --from commonmark \
        --to markdown \
        ${configMD} |


      # Convert Pandoc markdown to HTML using our own template and css files
      # where available. --sandbox is passed for extra security by default
      # with an optional override to disable it.
      pandoc \
       --from markdown \
       --to html \
       --metadata title="${title}" \
       --toc \
       --standalone \
    ''
    + optionalString embedResources ''--self-contained \''
    + optionalString sandboxing ''--sandbox \''
    + optionalString (templatePath != null) ''--template ${templatePath} \''
    + optionalString (styleSheetPath != null) ''--css ${ndg-stylesheet.override {inherit styleSheetPath;}} \''
    + optionalString (codeThemePath != null) ''--highlight-style ${codeThemePath} \''
    + "-o $out/share/doc/index.html"
  )
