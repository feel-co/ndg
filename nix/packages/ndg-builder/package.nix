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
  projects ? [],
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
  assert args ? specialArgs -> args ? rawModules || projects != [];
  assert assertMsg (isList stylesheets) "The stylesheets option is now additive, and takes a list instead";
  assert assertMsg (args ? evaluatedModules -> !(args ? rawModules)) "evaluatedModules and rawModules are mutually exclusive";
  assert assertMsg (!buildZim || zimId != null) "When buildZim is true, `zimId` must be set.";
  assert assertMsg (!buildZim || zimIllustration != null) "When buildZim is true, `zimIllustration` must be set.";
  assert assertMsg (!buildZim || lib.strings.hasSuffix ".png" zimIllustration) "When buildZim is true, `zimIllustration` must end in .png.";
  assert assertMsg (!buildZim || creator != null) "When buildZim is true, `creator` must be set.";
  assert assertMsg (!buildZim || publisher != null) "When buildZim is true, `publisher` must be set.";
  assert assertMsg (!buildZim || builtins.stringLength zimLanguage == 3) "When buildZim is true, `zimLanguage` must be a 3-character ISO639-3 language code."; let
    inherit (lib.strings) optionalString;
    inherit (lib.attrsets) filterAttrs;

    nonNullAttrs = filterAttrs (_: value: value != null);

    fetchGitSource = git:
      pkgs.fetchgit (nonNullAttrs {
        inherit (git) url rev;
        hash = git.hash or git.sha256;
        fetchSubmodules = git.fetchSubmodules or false;
        deepClone = git.deepClone or false;
        leaveDotGit = git.leaveDotGit or false;
      });

    githubRepoPath = git: let
      url = lib.strings.removeSuffix ".git" git.url;
    in
      if lib.strings.hasPrefix "https://github.com/" url
      then "${url}/blob/${git.rev}"
      else repoPath;

    mkTransformOptions = projectName: projectBasePath: projectRepoPath: opt:
      opt
      // {
        declarations = let
          inherit (lib) hasPrefix removePrefix pipe;
          basePathStr = toString projectBasePath;
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
                  url = "${projectRepoPath}/${x}";
                  name = "<${projectName}/${x}>";
                })
              ]
            else if decl == "lib/modules.nix"
            then {
              url = "https://github.com/NixOS/nixpkgs/blob/master/${decl}";
              name = "<nixpkgs/lib/modules.nix>";
            }
            else decl)
          opt.declarations;
      };

    defaultProject = {
      name = moduleName;
      title = "${moduleName} Options";
      slug = "options";
      version = null;
      inherit
        rawModules
        evaluatedModules
        specialArgs
        moduleArgs
        checkModules
        warningsAreErrors
        optionsDocArgs
        variablelistId
        basePath
        repoPath
        transformOptions
        ;
    };

    effectiveProjects =
      if projects == []
      then [defaultProject]
      else projects;

    evalProject = project: let
      name = project.name or moduleName;
      title = project.title or "${name} Options";
      version = project.version or null;
      slug =
        project.slug or (
          if version == null
          then "options/${name}"
          else "options/${name}/${version}"
        );
      src =
        project.src or (
          if project ? git
          then fetchGitSource project.git
          else null
        );
      projectRawModules =
        if project ? modules
        then
          if builtins.isFunction project.modules
          then project.modules src
          else project.modules
        else if project ? rawModules
        then
          if builtins.isFunction project.rawModules
          then project.rawModules src
          else project.rawModules
        else rawModules;
      projectModuleArgs =
        (project.moduleArgs or moduleArgs)
        // optionalAttrs (src != null) {
          inherit src;
          source = src;
        };
      projectSpecialArgs = project.specialArgs or specialArgs;
      projectEvaluatedModules =
        project.evaluatedModules or (lib.evalModules {
          modules =
            projectRawModules
            ++ [
              {
                options._module.args = lib.options.mkOption {
                  internal = true;
                };
                config._module = {
                  check = project.checkModules or checkModules;
                  args = projectModuleArgs;
                };
              }
            ];
          specialArgs = projectSpecialArgs;
        });
      projectBasePath =
        project.basePath or (
          if src != null
          then src
          else basePath
        );
      projectRepoPath =
        project.repoPath or (
          if project ? git
          then githubRepoPath project.git
          else repoPath
        );
      projectTransformOptions =
        project.transformOptions or (mkTransformOptions name projectBasePath projectRepoPath);
      projectOptionsDocArgs = project.optionsDocArgs or optionsDocArgs;
      projectOptionsJSON =
        (nixosOptionsDoc (
          {
            transformOptions = projectTransformOptions;
            warningsAreErrors = project.warningsAreErrors or warningsAreErrors;
            variablelistId = project.variablelistId or "${name}-options";
          }
          // (removeAttrs projectOptionsDocArgs ["options"])
          // {inherit (projectEvaluatedModules) options;}
        ))
        .optionsJSON;
    in
      {
        inherit title slug;
        path = "${projectOptionsJSON}/share/doc/nixos/options.json";
      }
      // optionalAttrs (version != null) {inherit version;};

    optionPages = map evalProject effectiveProjects;

    ndgConfig = writers.writeTOML "ndg.toml" (mergeAttrsList [
      {
        # Core Options
        inherit title;
        output_dir = placeholder "out";
        search.enable = generateSearch;
        highlight_code = highlightCode;
        sidebar.options.depth = optionsDepth;
        module_options_pages = optionPages;
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
