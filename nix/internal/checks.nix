{
  lib,
  pkgs,
  nixpkgs,
  system ? builtins.currentSystem,
  ndg-builder,
  nrd,
}: let
  sampleOptions = pkgs.writers.writeJSON "options.json" {
    "services.frobnicator.types.<name>.enable" = {
      declarations = ["nixos/modules/services/frobnicator.nix"];
      description = "Whether to enable the frobnication of this (`<name>`) type.";
      loc = ["services" "frobnicator" "types" "<name>" "enable"];
      readOnly = false;
      type = "boolean";
    };
  };
in {
  nixos = ndg-builder.override {
    evaluatedModules = import "${nixpkgs}/nixos/lib/eval-config.nix" {
      inherit lib system;
      modules = [
        ({modulesPath, ...}: {
          imports = ["${modulesPath}/profiles/minimal.nix"];
          boot.loader.grub.enable = false;
          fileSystems."/".device = "nodev";
          nixpkgs.hostPlatform = system;
          system.stateVersion = "26.11";
        })
      ];
    };
  };

  mermaid = let
    site = ndg-builder.override {
      mermaid = true;
      inputDir = pkgs.writeTextDir "diagram.md" ''
        # Diagram

        ```mermaid
        flowchart LR
          a --> b
        ```
      '';
    };
  in
    pkgs.runCommandLocal "ndg-mermaid" {} ''
      grep -F '<pre class="mermaid">flowchart LR' ${site}/diagram.html
      grep -F 'data-mermaid-src="assets/mermaid.min.js"' ${site}/diagram.html
      test -s ${site}/assets/mermaid.min.js
      test -s ${site}/assets/mermaid-init.js
      touch $out
    '';

  nrd-options-commonmark = pkgs.testers.runNixOSTest {
    name = "ndg-nrd-options-commonmark";

    containers.machine = _: {
      virtualisation.diskSize = 512;
    };

    testScript = /* py */ ''
      machine.start()
      machine.succeed("cat > /tmp/manpage-urls.json <<'JSON'\n{}\nJSON\n")
      machine.succeed("cp ${sampleOptions} /tmp/options.json")

      machine.succeed("${nrd}/bin/nrd options commonmark --manpage-urls /tmp/manpage-urls.json --revision local /tmp/options.json /tmp/ndg-default.md")
      machine.succeed("${pkgs.nixos-render-docs}/bin/nixos-render-docs options commonmark --manpage-urls /tmp/manpage-urls.json --revision local /tmp/options.json /tmp/nrd-default.md")
      machine.succeed("cmp /tmp/ndg-default.md /tmp/nrd-default.md")

      machine.succeed("${nrd}/bin/nixos-render-docs options commonmark --manpage-urls /tmp/manpage-urls.json --revision local --anchor-style legacy --anchor-prefix opt- /tmp/options.json /tmp/ndg-legacy.md")
      machine.succeed("${pkgs.nixos-render-docs}/bin/nixos-render-docs options commonmark --manpage-urls /tmp/manpage-urls.json --revision local --anchor-style legacy --anchor-prefix opt- /tmp/options.json /tmp/nrd-legacy.md")
      machine.succeed("cmp /tmp/ndg-legacy.md /tmp/nrd-legacy.md")
    '';
  };
}
