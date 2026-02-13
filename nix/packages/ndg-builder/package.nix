{
  pkgs,
  lib,
  # Build Dependencies
  ndg,
  runCommandLocal,
  nixosOptionsDoc,
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
  scripts ? [],
  inputDir ? null,
  stylesheet ? null,
  verbose ? true,
  manpageUrls ? null,
  optionsDepth ? 2,
  generateSearch ? true,
  highlightCode ? true,
} @ args: let
  inherit (lib.asserts) assertMsg;
in
  # TODO explain this one
  assert args ? specialArgs -> args ? rawModules;
  assert assertMsg (args ? evaluatedModules -> !(args ? rawModules)) "evaluatedModules and rawModules are mutually exclusive"; let
    inherit (lib.strings) optionalString concatMapStringsSep;

    configJSON =
      (nixosOptionsDoc (
        {inherit transformOptions warningsAreErrors variablelistId;}
        // (removeAttrs optionsDocArgs ["options"])
        // {inherit (evaluatedModules) options;}
      ))
      .optionsJSON;
  in
    runCommandLocal "ndg-builder" {
      nativeBuildInputs = [ndg];
      meta = {inherit description;};
    } (
      ''
        mkdir -p $out
        config_path="$TMPDIR/ndg-builder.toml"
        {
          echo "output_dir = \"$out\""
      ''
      + optionalString (inputDir != null) ''
          echo "input_dir = \"${toString inputDir}\""
      ''
      + optionalString (inputDir == null) ''
          echo "module_options = \"${configJSON}/share/doc/nixos/options.json\""
      ''
      + ''
        } > "$config_path"

        ndg --config-file "$config_path" ${optionalString verbose "--verbose"} html \
          --jobs $NIX_BUILD_CORES --output-dir "$out" --title "${title}" \
          --module-options "${configJSON}/share/doc/nixos/options.json" \
      ''
      + optionalString (scripts != []) ''${concatMapStringsSep "-" (x: "--script ${x}") scripts} \''
      + optionalString (stylesheet != null) ''--stylesheet ${toString stylesheet} \''
      + optionalString (inputDir != null) ''--input-dir ${inputDir} \''
      + optionalString (manpageUrls != null) ''--manpage-urls ${manpageUrls} \''
      + optionalString generateSearch ''--generate-search \''
      + optionalString highlightCode ''--highlight-code \''
      + "--options-depth ${toString optionsDepth}"
    )
