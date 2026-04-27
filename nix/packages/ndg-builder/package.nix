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
} @ args: let
  inherit (builtins) isList;
  inherit (lib.modules) mkIf;
  inherit (lib.asserts) assertMsg;
in
  # TODO explain this one
  assert args ? specialArgs -> args ? rawModules;
  assert assertMsg (isList stylesheets) "The stylesheets option is now additive, and takes a list instead";
  assert assertMsg (args ? evaluatedModules -> !(args ? rawModules)) "evaluatedModules and rawModules are mutually exclusive"; let
    inherit (lib.strings) optionalString;

    configJSON =
      (nixosOptionsDoc (
        {inherit transformOptions warningsAreErrors variablelistId;}
        // (removeAttrs optionsDocArgs ["options"])
        // {inherit (evaluatedModules) options;}
      ))
      .optionsJSON;

    ndgConfig = writers.writeToml "ndg.toml" {
      # Core Options
      inherit title;
      output_dir = builtins.placeholder "out";
      input_dir = mkIf (inputDir != null) (toString inputDir);
      module_options = mkIf (inputDir != null) "\"${configJSON}/share/doc/nixos/options.json\""; # TODO: check if there are options
      search.enable = generateSearch;
      highlight_code = highlightCode;
      sidebar.options.depth = optionsDepth;
      manpage_urls_path = mkIf (manpageUrls != null) manpageUrls;
      stylesheet_paths = mkIf (stylesheets != []) stylesheets;
      script_paths = mkIf (stylesheets != []) scripts;
    };
  in
    runCommandLocal "ndg-builder" {
      nativeBuildInputs = [ndg];
      meta = {inherit description;};
    } (
      optionalString (inputDir == null) ''
        ndg --config-file "${ndgConfig}" ${optionalString verbose "--verbose"} html \
          --jobs $NIX_BUILD_CORES --output-dir "$out"
      ''
    )
