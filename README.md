<h1 align="center">
  <br>
  NDG: Not a Docs Generator
  <br>
</h1>

## What is it?

> [!WARNING]
> ndg is currently in the process of being rewritten. Until the `v2` branch is
> merged into main, nothing is final. Please proceed with caution.

**ndg** is a fully customizable tool to automatically generate HTML documents
including but not limited to project documentation from options you use in any
set of modules.

### Features

- **Markdown to HTML conversion** with support for all common markdown features
- **Automatic table of contents** generation from document headings
- **Search functionality** to quickly find content across documents
- **NixOS options support** to generate documentation from `options.json`
- **Fully customizable templates** to match your project's style
- **Multi-threading support** for fast generation of large documentation sets

## Usage

```
Usage: ndg [OPTIONS] --input <INPUT> --output <OUTPUT> --title <TITLE>

Options:
  -i, --input <INPUT>
          Path to the directory containing markdown files
  -o, --output <OUTPUT>
          Output directory for generated documentation
  -p, --jobs <JOBS>
          Number of threads to use for parallel processing
  -v, --verbose
          Enable verbose debug logging
  -t, --template <TEMPLATE>
          Path to custom template file
  -s, --stylesheet <STYLESHEET>
          Path to custom stylesheet
      --script <SCRIPT>
          Path to custom Javascript file to include. This can be specified multiple times to create multiple script tags in order
  -T, --title <TITLE>
          Title of the documentation. Will be used in various components via the templating options
  -f, --footer <FOOTER>
          Footer text for the documentation
  -j, --module-options <MODULE_OPTIONS>
          Path to a JSON file containing module options in the same format expected by nixos-render-docs
      --options-depth <OPTIONS_TOC_DEPTH>
          Depth of parent categories in options TOC [default: 2]
      --manpage-urls <MANPAGE_URLS>
          Path to manpage URL mappings JSON file
  -h, --help
          Print help
  -V, --version
          Print version
```

You must give ndg a **input directory** containing your markdown files
(`--input`), an **output directory** (`--output`) to put the created files, and
a **title** (`--title`) for basic functionality.

For example:

```bash
ndg -i ./ndg-example/docs -o ./ndg-example/html -T "Awesome Nix Project"
```

where `./ndg-example/docs` contains markdown files that you would like to
convert, and `./ndg-example/html` is the result directory.

Optionally you may pass an `options.json` to also render an `options.html` page
for a listing of your module options.

### Customization

ndg supports various customization options to tailor the output to your needs:

#### Custom Templates

You can provide your own HTML template using the `--template` option:

```bash
ndg -i ./docs -o ./html -T "Awesome Project" -t ./my-template.html
```

Custom templates use the Tera templating engine, so your templates should use
Tera syntax:

- `{{ title }}` - Document title
- `{{ content|safe }}` - Main content area (unescaped HTML)
- `{{ toc|safe }}` - Table of contents (unescaped HTML)
- `{{ doc_nav|safe }}` - Navigation links to other documents (unescaped HTML)
- `{{ has_options|safe }}` - Conditional display for options page link

#### Custom Stylesheet

You can provide your own CSS stylesheet using the `--stylesheet` option:

```bash
ndg -i ./docs -o ./html -T "My Project" -s ./my-styles.css
```

ndg is able to compile SCSS as well, should you choose to pass it a file with
the `.scss` extension. The stylesheet will be compiled in runtime, and placed as
a `.css` file in the output directory.

#### NixOS Options

To include a page listing NixOS options, provide a path to your `options.json`
file:

```bash
ndg -i ./docs -o ./html -T "My Project" -j ./options.json
```

[nvf]: https://github.com/notashelf/nvf

The `options.json` should be in the standard NixOS format. An example from
[nvf], created via `pkgs.nixosOptionsDoc`.

```JSON
{
  "vim.withRuby": {
    "declarations": [
      {
        "name": "<nvf/modules/wrapper/environment/options.nix>",
        "url": "https://github.com/NotAShelf/nvf/blob/main/modules/wrapper/environment/options.nix"
      }
    ],
    "default": {
      "_type": "literalExpression",
      "text": "true"
    },
    "description": "Whether to enable Ruby support in the Neovim wrapper.\n.",
    "example": {
      "_type": "literalExpression",
      "text": "true"
    },
    "loc": ["vim", "withRuby"],
    "readOnly": false,
    "type": "boolean"
  }
}
```

## Building from Source

ndg is written in Rust, and a derivation is available.

```bash
nix build .#ndg
```

Alternatively, it can be built inside the dev shell using direnv or
`nix
develop.` Enter a shell, and `cargo build`.

## License

ndg is available under **Mozilla Public License, version 2.0**. Please see the
[LICENSE](./LICENSE) file for more details.
