{
  flake-parts-lib,
  self,
  ...
}: {
  options.perSystem = flake-parts-lib.mkPerSystemOption ({
    config,
    pkgs,
    lib,
    ...
  }: let
    inherit (lib.modules) mkOption;
    inherit (lib.types) package attrs listOf anything bool;

    cfg = config.perSystem.ndg;
  in {
    options.ndg = {
      checkModules = mkOption {
        type = bool;
        default = false;
        description = ''
          Whether to check modules passed to evaluatedModules
        '';
      };

      specialArgs = mkOption {
        type = attrs;
        default = {};
        description = ''
          Special arguments to pass to lib.evalModules. This is useful
          for passing arguments to the modules that are being evaluated.
        '';
      };

      rawModules = mkOption {
        type = listOf anything;
        default = [
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
        ];

        description = ''
          A list of **raw** modules containing options to be passed to
          `perSystem.ndg.package` to build option documentation. This
          is the equivalent of passing `rawModules` to `ndg-builder`
          with an override while using the package.
        '';
      };

      evaluatedModules = mkOption {
        type = attrs;
        default = lib.evalModules {
          modules = builtins.concatLists [
            cfg.rawModules
            [{_module.check = cfg.checkModules;}]
          ];

          inherit (cfg) specialArgs;
        };

        description = ''
          The result of lib.evalModules applied to a list of modules
          containing some options to document.
        '';
      };

      package = mkOption {
        type = package;
        default = self.packages.ndg-builder;
        defaultText = "<ndg-builder package from Flake outputs>";
        description = ''
          The builder derivation that will be used by default. You may pass
          a pacakge with your own overrides (e.g. stylesheet) if you wish to
          alter the defaults.
        '';
      };

      documentationPackage = mkOption {
        type = package;
        default = {};
        readOnly = true;
        description = "The resulting package";
      };
    };

    config.perSystem.ndg = {
      # TODO: assertions on cfg
      documentation = cfg.package.override {
        inherit (cfg) rawModules;
      };
    };
  });
}
