{
  pkgs,
  lib,
  # Build Dependencies
  ndg,
  runCommandLocal,
  nixosOptionsDoc,
  writers,
  # Options
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
  scrubDerivations ? namePrefix: pkgSet: let
    inherit (builtins) isAttrs;
    inherit (lib.attrs) mapAttrs optionalAttrs isDerivation;
  in
    mapAttrs (
      name: value: let
        wholeName = "${namePrefix}.${name}";
      in
        if isAttrs value
        then
          scrubDerivations wholeName value
          // optionalAttrs (isDerivation value) {
            inherit (value) drvPath;
            outPath = "\${${wholeName}}";
          }
        else value
    )
    pkgSet,
  moduleArgs ? {pkgs = lib.modules.mkForce (scrubDerivations "pkgs" pkgs);},
  specialArgs ? {},
  evaluatedModules ?
    lib.evalModules {
      modules =
        rawModules
        ++ [
          {
            options._module.args = lib.options.mkOption {
              internal = true;
            };
            config._module = {
              check = checkModules;
              args = moduleArgs;
            };
          }
        ];
      inherit specialArgs;
    },
  warningsAreErrors ? true,
  moduleName ? "myModule",
  variablelistId ? "${moduleName}-options",
  basePath ? ./.,
  repoPath ? "https://github.com/username/repo/blob/main",
  transformOptions ? opt:
    opt
    // {
      declarations = let
        inherit (lib) hasPrefix removePrefix pipe;
        basePathStr = toString basePath;
      in
        map
        (decl: let
          declStr = toString decl;
        in
          if hasPrefix basePathStr declStr
          then
            pipe declStr [
              (removePrefix basePathStr)
              (removePrefix "/")
              (x: {
                url = "${repoPath}/${x}";
                name = "<${moduleName}/${x}>";
              })
            ]
          else if decl == "lib/modules.nix"
          then {
            url = "https://github.com/NixOS/nixpkgs/blob/master/${decl}";
            name = "<nixpkgs/lib/modules.nix>";
          }
          else decl)
        opt.declarations;
    },
  # Builder configuration
  title ? "Site created by NDG",
  description ? "Generate static site docs of nix options",
  optionsDocArgs ? {},
  inputDir ? null,
  stylesheets ? [],
  scripts ? [],
  verbose ? true,
  manpageUrls ? null,
  optionsDepth ? 2,
  generateSearch ? true,
  highlightCode ? true,
  extraConfig ? {},
  # ZIM archive generation
  buildZim ? false,
  zimId ? null,
  zimLanguage ? "eng",
  zimIllustration ? null,
  zimTags ? ["devdocs" "nix"],
  creator ? null,
  publisher ? null,
  source ? null,
} @ args: let
  inherit (builtins) isList;
  inherit (lib.attrsets) optionalAttrs mergeAttrsList;
  inherit (lib.asserts) assertMsg;
in
  # TODO explain this one
  assert args ? specialArgs -> args ? rawModules;
  assert assertMsg (isList stylesheets) "The stylesheets option is now additive, and takes a list instead";
  assert assertMsg (args ? evaluatedModules -> !(args ? rawModules)) "evaluatedModules and rawModules are mutually exclusive";
  assert assertMsg (!buildZim || zimId != null) "When buildZim is true, `zimId` must be set.";
  assert assertMsg (!buildZim || zimIllustration != null) "When buildZim is true, `zimIllustration` must be set.";
  assert assertMsg (!buildZim || lib.strings.hasSuffix ".png" zimIllustration) "When buildZim is true, `zimIllustration` must end in .png.";
  assert assertMsg (!buildZim || creator != null) "When buildZim is true, `creator` must be set.";
  assert assertMsg (!buildZim || publisher != null) "When buildZim is true, `publisher` must be set.";
  assert assertMsg (!buildZim || builtins.stringLength zimLanguage == 3) "When buildZim is true, `zimLanguage` must be a 3-character ISO639-3 language code."; let
    inherit (lib.strings) optionalString;

    configJSON =
      (nixosOptionsDoc (
        {inherit transformOptions warningsAreErrors variablelistId;}
        // (removeAttrs optionsDocArgs ["options"])
        // {inherit (evaluatedModules) options;}
      ))
      .optionsJSON;

    ndgConfig = writers.writeTOML "ndg.toml" (mergeAttrsList [
      {
        # Core Options
        inherit title;
        output_dir = placeholder "out";
        search.enable = generateSearch;
        highlight_code = highlightCode;
        sidebar.options.depth = optionsDepth;
        module_options = "${configJSON}/share/doc/nixos/options.json"; # TODO: check if there are options
      }
      (optionalAttrs (inputDir != null) {input_dir = inputDir;})
      (optionalAttrs (manpageUrls != null) {manpage_urls_path = manpageUrls;})
      (optionalAttrs (stylesheets != []) {stylesheet_paths = stylesheets;})
      (optionalAttrs (scripts != []) {script_paths = scripts;})
      (optionalAttrs (extraConfig != {}) extraConfig)
    ]);
  in
    runCommandLocal "ndg-builder" {
      nativeBuildInputs = [ndg];
      meta = {inherit description;};
    } ''
      ndg --config-file "${ndgConfig}" ${optionalString verbose "--verbose"} html \
        --jobs $NIX_BUILD_CORES --output-dir "$out"

      ${optionalString buildZim ''
        workDir=$(mktemp -d)
        cp -r "$out"/* "$workDir/"
        cp "${zimIllustration}" "$workDir/icon.png"

        ${pkgs.zim-tools}/bin/zimwriterfs ${builtins.concatStringsSep " " [
          "--welcome 'index.html'"
          "--illustration 'icon.png'"
          "--illustration 'icon.png'"
          "--language '${zimLanguage}'"
          "--name '${zimId}'"
          "--title '${title}'"
          "--description '${description}'"
          "--creator '${creator}'"
          "--publisher '${publisher}'"
          "--tags '${builtins.concatStringsSep ";" zimTags}'"
          (optionalString (source != null) "--source '${source}'")
          "--scraper 'ndg'"
          "\"$workDir\" \"$out/${zimId}.zim\""
        ]}
      ''}
    ''
