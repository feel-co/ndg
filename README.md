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
  -j, --options-json <OPTIONS_JSON>  Path to options.json file
  -i, --input <INPUT>                Path to markdown files directory
  -o, --output <OUTPUT>              Output directory for generated documentation
  -t, --template <TEMPLATE>          Path to custom template file
  -s, --stylesheet <STYLESHEET>      Path to custom stylesheet
  -T, --title <TITLE>                Title of the documentation
  -p, --jobs <JOBS>                  Number of threads to use for parallel processing
  -v, --verbose                      Enable verbose logging
  -h, --help                         Print help
  -V, --version                      Print version
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
ndg -i ./docs -o ./html -T "My Project" -t ./my-template.html
```

The template should include the following placeholders:

- `{{title}}` - Document title
- `{{content}}` - Main content area
- `{{toc}}` - Table of contents
- `{{doc_nav}}` - Navigation links to other documents
- `{{has_options}}` - Conditional display for options page link

#### Custom Stylesheet

You can provide your own CSS stylesheet using the `--stylesheet` option:

```bash
ndg -i ./docs -o ./html -T "My Project" -s ./my-styles.css
```

#### NixOS Options

To include a page listing NixOS options, provide a path to your `options.json`
file:

```bash
ndg -i ./docs -o ./html -T "My Project" -j ./options.json
```

The `options.json` should be in the standard NixOS format:

```JSON
{
  "option.name": {
    "type": "string",
    "description": "Description of the option",
    "default": "default value",
    "example": "example value",
    "declared": "module path"
  }
}
```

## Building from Source

ndg is written in Rust. To build it from source:

- Make sure you have Rust installed (https://rustup.rs/)
- Clone the repository and enter a dev shell (`nix develop`)
- Build with `cargo build` (Nix packaging not available yet)

## License

ndg is available under **Mozilla Public License, version 2.0**. Please see the
[LICENSE](./LICENSE) file for more details.
