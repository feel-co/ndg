{
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
  specialArgs ? {},
  evaluatedModules ?
    lib.evalModules {
      modules = rawModules ++ [{_module.check = checkModules;}];
      inherit specialArgs;
    },
  # Builder configuration
  title ? "My Option Documentation",
  optionsDocArgs ? {},
  scripts ? [],
  inputDir ? null,
  stylesheet ? null,
  verbose ? false,
} @ args: let
  inherit (lib.asserts) assertMsg;
in
  # TODO explain this one
  assert args ? specialArgs -> args ? rawModules;
  assert assertMsg (args ? evaluatedModules -> !(args ? rawModules)) "evaluatedModules and rawModules are mutually exclusive"; let
    inherit (lib.strings) optionalString concatMapStringsSep;

    configJSON =
      (nixosOptionsDoc (
        (removeAttrs optionsDocArgs ["options"])
        // {inherit (evaluatedModules) options;}
      ))
      .optionsJSON;
  in
    runCommandLocal "ndg-builder" {nativeBuildInputs = [ndg];} ''
      mkdir -p $out

      ndg options --jobs $NIX_BUILD_CORES --output-dir "$out" --title "${title}" \
        --module-options "${configJSON}/share/doc/nixos/options.json" \
        ${optionalString (inputDir != null) ''--input-dir ${inputDir} \''}
        ${optionalString (scripts != []) ''${concatMapStringsSep "-" (x: "--script ${x}") scripts} \''}
        ${optionalString (stylesheet != null) ''--stylesheet ${toString stylesheet} \''}
        ${optionalString verbose ''--verbose \''}
    ''
