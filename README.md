<h1 align="center">
  <br>
  NDG: Not a Docs Generator
  <br>
</h1>

## What is it?

`ndg` is a way to automatically generate an HTML document containing all the
options you use in any set of modules.

This flake exposes a package (`packages.<system>.ndg-builder` a.k.a.
`packages.<system>.default`), as well as an overlay (`overlays.default`) to
allow accessing `ndg-builder` in another nixpkgs instance.

## Usage

You can override the exposed package with the following options:

- `rawModules`: a list of modules containing `options` to document. For example:

```nix
[
  {
    options.hello = lib.mkOption {
      default = "world";
      description = "Example option.";
      type = lib.types.str;
    };
  }
]
```

- `specialArgs`: the `specialArgs` to pass through to the `lib.evalModules`
  call.
- `evaluatedModules`: the result of `lib.evalModules` applied to a list of
  modules containing some `options` to document. For example:

```nix
lib.evalModules {
  modules = [
    {
      options.hello = lib.mkOption {
        default = "world";
        description = "Example option.";
        type = lib.types.str;
      };
    }
  ];
}
```

This includes anything that uses `lib.evalModules` underneath, such as a NixOS,
Home Manager, or Nix-Darwin configuration. For example, in the context of a
flake:

```
self.nixosConfigurations.myHost
```

<!-- deno-fmt-ignore-start -->

> [!NOTE]
> `rawModules` and `evaluatedModules` are **mutually exclusive**.

<!-- deno-fmt-ignore-end -->

[pandoc template]: https://pandoc.org/MANUAL.html#templates
[pandoc syntax highlighting file]: https://pandoc.org/MANUAL.html#syntax-highlighting

- `title`: the title of your documentation page
- `sandboxing`: whether to pass `--sandbox` to Pandoc
- `embedResources`: whether `--self-contained` should be passed to Pandoc in
  order to generate an entirely self-contained HTML document
- `templatePath`: path to a [pandoc template]
- `styleSheetPath`: path to a Sassy CSS (SCSS) file that will compile to CSS
- `codeThemePath`: path to a [pandoc syntax highlighting file] (note that it
  must be JSON with a `.theme` extension)
- `generateLinkAnchors`: whether to generate anchors next to links
- `optionsDocArgs`: additional arguments to pass to the `nixosOptionsDoc`
  package
