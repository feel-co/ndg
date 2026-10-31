{
  lib,
  pkgs,
  nixpkgs,
  system ? builtins.currentSystem,
  ndg-builder,
  nrd,
}: let
  sampleOptions = pkgs.writers.writeJSON "options.json" {
    "services.foo.enable" = {
      declarations = ["nixos/modules/services/foo.nix"];
      definitions = [
        {
          name = "<foo/modules/default.nix>";
          url = "https://example.invalid/foo/modules/default.nix";
        }
      ];
      default = {
        _type = "literalExpression";
        text = "false";
      };
      description = "Whether to enable {command}`fooctl`. See [the manual](https://example.invalid/manual) and [](#opt-services.foo.mode).";
      example = {
        _type = "literalMD";
        text = "`true`";
      };
      loc = ["services" "foo" "enable"];
      readOnly = false;
      relatedPackages = "{manpage}`foo(1)` and {file}`/nix/store/example`";
      type = "boolean";
    };
    "services.foo.mode" = {
      declarations = ["/nix/store/abc-source/nixos/modules/services/foo.nix"];
      default = {
        _type = "literalExpression";
        text = "\"auto\"";
      };
      description = {
        _type = "mdDoc";
        text = "Mode for {option}`services.foo.enable` using {env}`FOO_MODE`, {var}`mode`, and {file}`/etc/foo.conf`.";
      };
      loc = ["services" "foo" "mode"];
      readOnly = true;
      type = "one of \"auto\", \"manual\"";
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

  nrd-options-commonmark = pkgs.testers.runNixOSTest {
    name = "ndg-nrd-options-commonmark";

    containers.machine = {
      virtualisation = {
        diskSize = 512;
        vlans = [];
      };
    };

    testScript =
      /*
      py
      */
      ''
        with subtest("prepare fixtures"):
          machine.start()
          machine.succeed("cat > /tmp/manpage-urls.json <<'JSON'\n{}\nJSON\n")
          machine.succeed("cp ${sampleOptions} /tmp/options.json")
          machine.succeed("printf '%s\\n' '.TH TEST 5' > /tmp/header.roff")
          machine.succeed("printf '%s\\n' '.SH END' > /tmp/footer.roff")

        with subtest("nrd commonmark output"):
          machine.succeed("${nrd}/bin/nrd options commonmark --manpage-urls /tmp/manpage-urls.json --revision local /tmp/options.json /tmp/ndg-default.md")
          machine.succeed("${pkgs.nixos-render-docs}/bin/nixos-render-docs options commonmark --manpage-urls /tmp/manpage-urls.json --revision local /tmp/options.json /tmp/nrd-default.md")
          machine.succeed("cmp /tmp/ndg-default.md /tmp/nrd-default.md")

        with subtest("nixos-render-docs commonmark alias"):
          machine.succeed("${nrd}/bin/nixos-render-docs options commonmark --manpage-urls /tmp/manpage-urls.json --revision local /tmp/options.json /tmp/ndg-alias.md")
          machine.succeed("cmp /tmp/ndg-alias.md /tmp/nrd-default.md")

        with subtest("legacy commonmark anchors"):
          machine.succeed("${nrd}/bin/nrd options commonmark --manpage-urls /tmp/manpage-urls.json --revision local --anchor-style legacy --anchor-prefix opt- /tmp/options.json /tmp/ndg-legacy.md")
          machine.succeed("${pkgs.nixos-render-docs}/bin/nixos-render-docs options commonmark --manpage-urls /tmp/manpage-urls.json --revision local --anchor-style legacy --anchor-prefix opt- /tmp/options.json /tmp/nrd-legacy.md")
          machine.succeed("cmp /tmp/ndg-legacy.md /tmp/nrd-legacy.md")

        with subtest("nrd manpage output"):
          machine.succeed("${nrd}/bin/nrd options manpage --revision local --header /tmp/header.roff --footer /tmp/footer.roff /tmp/options.json /tmp/ndg-options.roff")
          machine.succeed("${pkgs.nixos-render-docs}/bin/nixos-render-docs options manpage --revision local --header /tmp/header.roff --footer /tmp/footer.roff /tmp/options.json /tmp/nrd-options.roff")
          machine.succeed("cmp /tmp/ndg-options.roff /tmp/nrd-options.roff")

        with subtest("nixos-render-docs manpage alias"):
          machine.succeed("${nrd}/bin/nixos-render-docs options manpage --revision local --header /tmp/header.roff --footer /tmp/footer.roff /tmp/options.json /tmp/ndg-options-alias.roff")
          machine.succeed("cmp /tmp/ndg-options-alias.roff /tmp/nrd-options.roff")
      '';
  };
}
