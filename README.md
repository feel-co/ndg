<h1 align="center">
  <br>
  NDG: Not a Docs Generator
  <br>
</h1>

## What is it?

> [!WARNING]
> ndg is currently in the process of being rewritten. Until the `v2` branch is
> merged into main, nothing is final. Please proceed with caution.

**ndg** is a fast, customizable and (relatively) sane tool for automatically
generating HTML documents including but not limited to project documentation
from Markdown and options you use in any set of modules.

### Features

- **Markdown to HTML conversion** with support for all[^1] Nixpkgs-flavored
  Commonmark features
- **Automatic table of contents** generation from document headings
  - **Title Anchors** - Automatically generates anchors for headings
- **Search functionality** to quickly find content across documents (can be
  disabled with `--generate-search false`)
- **Nix module options support** to generate documentation from `options.json`
- **Fully customizable templates** to match your project's style
- **Multi-threading support** for fast generation of large documentation sets

[^1]: This is a best-effort. Regular commonmark is fully supported, but Nixpkgs
    additions may sometimes break due to the lack of a clear specification.
    Please open an issue if this is the case for you.

## Usage

```
Usage: ndg [OPTIONS]

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
      --template-dir <TEMPLATE_DIR>
          Path to directory containing template files (default.html, options.html, etc.) For missing required files, ndg will fall back to its internal templates
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
          Depth of parent categories in options TOC
      --manpage-urls <MANPAGE_URLS>
          Path to manpage URL mappings JSON file
  -c, --config <CONFIG_FILE>
          Path to configuration file (TOML or JSON)
  -h, --help
          Print help
  -V, --version
          Print version
```

You must give ndg a **input directory** containing your markdown files
(`--input`), an **output directory** (`--output`) to put the created files, and
a **title** (`--title`) for basic functionality. Alternatively, those can all be
read from a config file, passed to ndg through the `--config` option.

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

You can provide your own HTML template using either the `--template` option for
a single template file:

```bash
ndg -i ./docs -o ./html -T "Awesome Project" -t ./my-template.html
```

Or use the `--template-dir` option to provide a directory containing multiple
templates:

```bash
ndg -i ./docs -o ./html -T "Awesome Project" --template-dir ./my-templates
```

When using a template directory, ndg will look for the following files:

- `default.html` - Main template for documentation pages
- `options.html` - Template for options page
- `search.html` - Template for search page
- `options_toc.html` - Template for options table of contents

Custom templates use the Tera templating engine, so your templates should use
Tera syntax. Below are the variables that ndg will attempt to replace.

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

#### Search Configuration

By default, ndg generates a search page and a search widget for your
documentation. You can disable this with:

```bash
ndg -i ./docs -o ./html -T "My Project" --generate-search false
```

This will prevent the creation of search.html and the search index data, and the
search widget won't appear in the navigation.

## Building from Source

ndg is written in Rust, and a derivation is available.

```bash
nix build .#ndg -Lv
```

Alternatively, it can be built inside the dev shell using direnv or
`nix develop.` Enter a shell, and `cargo build`.

## License

ndg is available under **Mozilla Public License, version 2.0**. Please see the
[LICENSE](./LICENSE) file for more details.
