{
  lib,
  # Build Dependencies
  runCommandLocal,
  pandoc,
  nixosOptionsDoc,
  ndg-stylesheet,
  # Options
  pandocCmd ? "pandoc",
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
  # Builder configuration
  title ? "My Option Documentation",
  templatePath ? ./assets/default-template.html,
  styleSheetPath ? ./assets/default-styles.scss,
  codeThemePath ? ./assets/default-syntax.theme,
  filters ? [],
  optionsDocArgs ? {},
  sandboxing ? true,
  embedResources ? false,
  generateLinkAnchors ? true,
  outPath ? "share/doc",
} @ args:
assert args ? specialArgs -> args ? rawModules;
assert args ? evaluatedModules -> !(args ? rawModules); let
  inherit (lib.strings) optionalString concatStringsSep;
  inherit (lib.lists) optionals;

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

  pandocArgs = {
    luaFilters = filters ++ optionals generateLinkAnchors [./assets/filters/anchor.lua];
    finalOutPath = "$out" + /${toString outPath}; # toString handles 'null' case
  };
in
  runCommandLocal "generate-option-docs.html" {nativeBuildInputs = [pandoc];} (
    ''
      mkdir -p ${pandocArgs.finalOutPath}

      ${optionalString genJsonDocs ''
        cp -vf ${configJSON}/share/doc/nixos/options.json $out/share/doc/options.json
      ''}

      # Convert to Pandoc markdown instead of using commonmark directly
      # as the former automatically generates heading IDs and TOC links.
      ${pandocCmd} \
        --from commonmark \
        --to markdown \
        ${configMD} |


      # Convert Pandoc markdown to HTML using our own template and css files
      # where available. --sandbox is passed for extra security by default
      # with an optional override to disable it.
      ${pandocCmd} \
       --from markdown \
       --to html \
       --metadata title="${title}" \
       --toc \
       --standalone \
    ''
    + optionalString (pandocArgs.luaFilters != []) ''${concatStringsSep " " (map (f: "--lua-filter=" + f) pandocArgs.luaFilters)} \''
    + optionalString embedResources ''--embed-resources --standalone \''
    + optionalString sandboxing ''--sandbox \''
    + optionalString (templatePath != null) ''--template ${templatePath} \''
    + optionalString (styleSheetPath != null) ''--css ${ndg-stylesheet.override {inherit styleSheetPath;}} \''
    + optionalString (codeThemePath != null) ''--highlight-style ${codeThemePath} \''
    + "-o ${pandocArgs.finalOutPath}/index.html"
  )
