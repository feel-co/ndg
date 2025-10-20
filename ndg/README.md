<!-- markdownlint-disable MD013 MD033 -->

<h1 align="center">
  <br>
  NDG: Not a Docs Generator
  <br>
</h1>

## What is it?

**ndg** is a fast, customizable and convenient utility for generating HTML
documents and manpages for your Nix related projects from Markdown and options
you use in any set of modules. It can be considered a more opinionated and more
specialized MdBook, focusing on Nix-related projects. Though it can also be used
outside of Nix projects.

### Features

We would like for ndg to be fast, but also flexible and powerful. It boasts
several features such as

- **Markdown to HTML and Manpage conversion** with support for Nixpkgs-flavored
  Commonmark features[^1].
- **Useful Markdown Companions**
  - **Automatic table of contents** generation from document headings
  - **Title Anchors** - Automatically generates anchors for headings
- **Search functionality** to quickly find content across documents
- **Nix module options support** to generate documentation from `options.json`
- **Fully customizable templates** to match your project's style fully
- **Multi-threading support** for fast generation of large documentation sets
- **Incredibly fast** documentation generation for all scenarios

[^1]: This is a best-effort. Regular commonmark is fully supported, but Nixpkgs
    additions may sometimes break due to the lack of a clear specification.
    Please open an issue if this is the case for you.

## Usage

NDG has several subcommands to handle different documentation tasks:

```console
$ ndg --help 
NDG: Not a Docs Generator

Usage: ndg [OPTIONS] [COMMAND]

Commands:
  init     Initialize a new NDG configuration file
  export   Export default templates to a directory for customization
  html     Process documentation and generate HTML
  manpage  Generate manpage from options
  help     Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose               Enable verbose debug logging
  -c, --config <CONFIG_FILE>  Path to configuration file (TOML or JSON)
  -h, --help                  Print help
  -V, --version               Print version
```

### Getting Started with Configuration

The easiest way to get started with ndg is to use the `init` command to generate
a default configuration file:

```bash
# Generate a default ndg.toml configuration file
$ ndg init

# Generate a JSON configuration file instead
$ ndg init --format json --output ndg.json

# Generate in a specific location
$ ndg init --output configs/ndg.toml
```

This will create a well-documented configuration file with all available options
and their default values. You can then edit this file to customize your
documentation generation process.

The generated configuration includes detailed comments for each option, making
it easy to understand what each setting does without needing to refer to the
documentation.

Once you have a configuration file, ndg will automatically detect it when run
from the same directory, or you can specify it explicitly with the `--config`
option:

```bash
# Run with automatically detected config
$ ndg html

# Run with explicitly specified config
$ ndg html --config path/to/ndg.toml
```

## Detailed Usage

### CLI Flags and Configuration Options

Customizing in NDG is done through two different methods: the configuration
file, and the CLI flags. The latter allows for ad-hoc overrides of specific
behaviour, such as data indexing for search and code highlighting. Two example
flags accepted by `ndg html` are:

- `--generate-search`: Enables search functionality (disabled by default if
  omitted).
- `--highlight-code`: Enables syntax highlighting for code blocks (disabled by
  default if omitted).

Example usage:

```bash
ndg html --input-dir ./docs --output-dir ./html --title "My Project" --generate-search
```

This will override the search data generation in, e.g., `config.toml`

#### Configuration Files

Configuration files (TOML or JSON) allow setting options persistently. Options
like `generate_search` can be set in the config file, but CLI flags will
override them.

Example `ndg.toml`:

```toml
input_dir = "docs"
output_dir = "html"
title = "My Project"
generate_search = true  # Can be overridden by CLI
```

#### Interactions Between CLI and Config

CLI flags always take precedence over config file settings.

For instance, if your config file has `generate_search = false`, but you run
`ndg html --generate-search`, search will be enabled.

If neither CLI nor config specifies an option, ndg uses sensible defaults (e.g.,
`generate_search` defaults to `false` if omitted from both; CLI flags override
config when present).

For a full list of options, see the help output with `ndg html --help`.

### Generating HTML Documents

The first subcommand in ndg's compartmentalized architecture is for HTML
generation. The `html` subcommand includes the following options:

```console
Options:
  -i, --input-dir <INPUT_DIR>
          Path to the directory containing markdown files
  -o, --output-dir <OUTPUT_DIR>
          Output directory for generated documentation
  -p, --jobs <JOBS>
          Number of threads to use for parallel processing
  -t, --template <TEMPLATE>
          Path to custom template file
      --template-dir <TEMPLATE_DIR>
          Path to directory containing template files. Templates override built-in ones (default.html, options.html, etc.)
  -s, --stylesheet <STYLESHEET>
          Path to custom stylesheet (can be specified multiple times)
      --script <SCRIPT>
          Path to custom JavaScript file (can be specified multiple times)
  -T, --title <TITLE>
          Title of the documentation. Will be used in various components via the templating options
  -f, --footer <FOOTER>
          Footer text for the documentation
  -j, --module-options <MODULE_OPTIONS>
          Path to a JSON file containing module options in the same format expected by nixos-render-docs
      --options-depth <OPTIONS_TOC_DEPTH>
          Depth of parent categories in options section in the sidebar
      --manpage-urls <MANPAGE_URLS>
          Path to manpage URL mappings JSON file
  -S, --generate-search
          Whether to generate search data and render relevant components
      --highlight-code
          Whether to enable syntax highlighting for code blocks
      --revision <REVISION>
          GitHub revision for linking to source files [default: local]
  -h, --help
          Print help
```

You must give ndg a **input directory** containing your markdown files
(`--input-dir`), an **output directory** (`--output-dir`) to put the created
files, and a **title** (`--title`) for basic functionality. Alternatively, those
can all be read from a config file, passed to ndg through the `--config` option.

For example:

```bash
# '-i' is the input directory and '-o' is the output directory
$ ndg html -i ./ndg-example/docs -o ./ndg-example/html -T "Awesome Nix Project"
```

For this example, `./ndg-example/docs` contains markdown files that you would
like to convert, and `./ndg-example/html` is the result directory after the
conversion.

> [!INFO]
> Optionally you may pass an `options.json` to also render an `options.html`
> page for a listing of your module options. See the
> [NixOS Options](#nixos-options) section below for more details.

### Generating Manpages

NDG allows for Man pages to be generated for your project. While the system is
not as robust as HTML generation due to the shortcomings of Troff the `manpage`
subcommand allows generating a manpage from your options:

```bash
$ ndg manpage -j ./options.json --output-file ./man/project.5 --title "My Project" --section 5
# => This will generate ./man/project.5 as your manpage.
```

For manpages, you **must provide** an `options.json` using `--module-options` or
`-j`. Since we do not convert arbitrary pages in the Manpages mode, this is
necessary to generate the main page containing module options.

### Customization

NDG is an opinionated CLI utility due to the reasons that warranted its
existence, but it still aims to support various colorful customization options
to tailor the output to your needs, regardless of how immodest they may be.

#### Custom Templates (HTML only)

By default, HTML output's design uses a small internal template hand-crafted for
NDG. This is suitable for certain usecases, but you might be looking for
something more. If this is the case for you, then you are encouraged to export
the default templates and change them as you wish.

We provide an `export` subcommand that exports the default templates into a
specified directory which you may modify as you wish, and use them using the
`--template-dir` flag in `ndg html`.

```bash
# Export all default templates to a directory
$ ndg export --output-dir my-templates

# Customize the templates as needed
# Edit my-templates/default.html, my-templates/default.css, etc.

# Then, use your custom templates
$ ndg html -i ./docs -o ./html -T "Awesome Project" --template-dir ./my-templates
```

You can also provide a single template file using the `--template` option:

```bash
ndg html -i ./docs -o ./html -T "Awesome Project" -t ./my-template.html
```

The `--template` flag will ONLY override the default template, which is used in
the generation of the converted Markdown pages. To modify search and options
pages, you may want to export the templates in full and modify them per your
needs.

When using a template directory, ndg will look for the following files:

- `default.html` - Main template for documentation pages
- `options.html` - Template for options page
- `search.html` - Template for search page
- `options_toc.html` - Template for options table of contents
- `default.css` - Default stylesheet
- `search.js` - Search functionality JavaScript

For any missing files, ndg will fall back to its internal templates.

Custom templates use the Tera templating engine, so your templates should use
Tera syntax. Below are the variables that ndg will attempt to replace.

- `{{ title }}` - Document title
- `{{ content|safe }}` - Main content area (unescaped HTML)
- `{{ toc|safe }}` - Table of contents (unescaped HTML)
- `{{ doc_nav|safe }}` - Navigation links to other documents (unescaped HTML)
- `{{ has_options|safe }}` - Conditional display for options page link
- `{{ footer_text }}` - Footer text specified via the --footer option
- `{{ generate_search }}` - Boolean indicating whether search functionality is
  enabled
- `{{ stylesheet_path }}` - Path to the stylesheet (automatically adjusted for
  subdirectories)
- `{{ main_js_path }}` - Path to the main JavaScript file (automatically
  adjusted for subdirectories)
- `{{ search_js_path }}` - Path to the search JavaScript file (automatically
  adjusted for subdirectories)
- `{{ index_path }}` - Path to the index page (automatically adjusted for
  subdirectories)
- `{{ options_path }}` - Path to the options page (automatically adjusted for
  subdirectories)
- `{{ search_path }}` - Path to the search page (automatically adjusted for
  subdirectories)
- `{{ custom_scripts|safe }}` - Custom script tags (unescaped HTML,
  automatically adjusted for subdirectories)

Each template can use these variables as needed. For example, in the
`default.html` template:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ title }}</title>
    <link rel="stylesheet" href="{{ stylesheet_path }}">
    <script defer src="{{ main_js_path }}"></script>
    {% if generate_search %}
    <script defer src="{{ search_js_path }}"></script>
    {% endif %}
  </head>
  <body>
    <header>
      <h1><a href="{{ index_path }}">{{ title }}</a></h1>
      {% if generate_search %}
      <div class="search-container">
        <input
          type="text"
          id="search-input"
          placeholder="Search documentation..."
        >
        <a href="{{ search_path }}" class="search-button">Advanced Search</a>
      </div>
      {% endif %}
    </header>

    <div class="container">
      <nav class="sidebar">
        {{ toc|safe }} {{ doc_nav|safe }} {% if has_options %}
        <div class="options-link">
          <a href="{{ options_path }}">Module Options</a>
        </div>
        {% endif %}
      </nav>

      <main class="content">
        {{ content|safe }}
      </main>
    </div>

    <footer>
      <p>{{ footer_text }}</p>
    </footer>

    {{ custom_scripts|safe }}
  </body>
</html>
```

Once exported, you can customize any of these files and use them with the
`--template-dir` option:

```bash
# Export templates and customize them
ndg export --output-dir my-templates

# Edit the templates as needed
# vim my-templates/default.html

# Use your custom templates
ndg html --template-dir my-templates --input-dir docs --output-dir build
```

This workflow is designed to aid developers avoid rebuilds, and to make sure
users always start with the latest default templates and can easily update them
when ndg is upgraded. Though, ndg will fall back to internal templates when none
are provided.

See the [templating documntation](./docs/TEMPLATING.md) for details about the
templating. If your use-case is not supported, feel free to request a feature!

### Template Customization Made Easy

```bash
# Export default templates to a 'templates' directory
ndg export

# Export to a custom directory
  ndg export --output-dir my-custom-templates

# Overwrite existing files if they exist
ndg export --force
```

#### Custom Stylesheet

NDG also bundles its own stylesheet as a part of the default templates. This is
meant to be a simple starting point, but you may easily override it with your
own stylesheets if you are not satisfied with the defaults. The `--stylesheet`
flag allows overriding the default CSS template with your own.

```bash
$ ndg html -i ./docs -o ./html -T "My Project" -s ./my-styles.css
#=> my-styles.css will be used in place of the 'default.css'
```

> [!WARNING]
> While overriding the CSS, it is strongly recommended that you do not partially
> override the files. This mechanism is intended for quick and dirty edits
> rather than full replacements, because the default HTML templates depend on
> CSS classes in the default stylesheet. If you would like to start fresh, you
> should consider exporting the default template, and overriding it in full.

NDG is able to compile SCSS as well, should you choose to pass it a file with
the `.scss` extension. The stylesheet will be compiled in runtime, and placed as
a `.css` file in the output directory.

#### NixOS Options

[nvf]: https://github.com/notashelf/nvf

One of the many purposes of NDG is to document options provided by the Nix
module system. This behaviour is similar to `nixos-render-docs` used in the
Nixpkgs manual, and is fully backwards compatible in terms of the file format
expected by the utility.

To include a page listing NixOS options, provide a path to your `options.json`
file:

```bash
$ ndg html -i ./docs -o ./html -T "My Project" -j ./options.json
#=> The output directory will contain an options.html with your option reference
```

While there is no clear standard, the `options.json` is expected to be in the
same "standard" format as the utility used by NixOS. In a traditional setup, you
can generate this file using `pkgs.nixosOptionsDoc` by providing it with
evaluated modules. If you are using the `ndg-builder` package instead of the raw
CLI, module evaluation is done for you automatically.

> [!TIP]
> An example from [nvf], created via `pkgs.nixosOptionsDoc`.
>
> ```json
> {
>   "vim.withRuby": {
>     "declarations": [
>       {
>         "name": "<nvf/modules/wrapper/environment/options.nix>",
>         "url": "https://github.com/NotAShelf/nvf/blob/main/modules/wrapper/environment/options.nix"
>       }
>     ],
>     "default": {
>       "_type": "literalExpression",
>       "text": "true"
>     },
>     "description": "Whether to enable Ruby support in the Neovim wrapper.\n.",
>     "example": {
>       "_type": "literalExpression",
>       "text": "true"
>     },
>     "loc": ["vim", "withRuby"],
>     "readOnly": false,
>     "type": "boolean"
>   }
> }
> ```

#### Search Configuration

Your documentation may get convoluted as it grows more comprehensive (because
you write such good docs!) so NDG includes the option to generate

1. A search page with full indexing
2. A search widget with limited functionality

to search across your pages and option references. This _used_ to be enabled by
default prior to version v2.4 but it now defaults to false to avoid invasive
behaviour and keep builds lightweight. You may re-enable the search page
generating by passing the `--generate-search` flag to `ndg html`

```bash
$ ndg html -i ./docs -o ./html -T "My Project"
#=> Search page will not be generated

$ ndg html -i ./docs -o ./html -T "My Project" --generate-search
#=> Search page will be generated
```

While the search is disabled, all search-related functionality will be skipped
during runtime.

## Using & Integrating with Nix

### Using NDG with Nix

NDG can be integrated into Nix builds to generate documentation for your
projects. Here are several ways to use ndg with Nix:

#### Installing from the Flake

The simplest way to install ndg is directly from its flake:

```bash
# Install to your profile
$ nix profile install github:feel-co/ndg#ndg # do not add the ndg-builder package!

# Or run without installing
$ nix run github:feel-co/ndg -- html -i ./docs -o ./result -T "My Project"
```

#### Using ndg-builder

The recommended way to generate documentation in Nix builds is to use the
included `ndg-builder` function, which properly handles all the complexities:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    ndg.url = "github:feel-co/ndg";
  };

  outputs = { self, nixpkgs, ndg, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      packages.${system}.docs = ndg.packages.${system}.ndg-builder.override {
        title = "My Project Documentation";
        inputDir = ./docs;
        stylesheet = ./assets/style.css;
        scripts = [ ./assets/custom.js ];

        # For module options
        rawModules = [
          # From a path
          ./modules/options.nix

          # From options
          ({
            options.example.option = pkgs.lib.mkOption {
              type = pkgs.lib.types.str;
              default = "value";
              description = "An example option";
            };
          })
        ];

        # Optional: set the depth of options TOC
        optionsDepth = 3;
      };
  };
}
```

#### Manual Approach Using `runCommandLocal`

If you need more control, you can use `runCommandLocal` directly:

```nix
let
  pkgs = import <nixpkgs> { };

  # Generate options JSON properly
  optionsJSON = (pkgs.nixosOptionsDoc {
    options = (pkgs.lib.evalModules {
      modules = [ ./modules/options.nix ];
    }).options;
  }).optionsJSON;

  # Generate documentation
  docs = pkgs.runCommandLocal "project-docs" {
    nativeBuildInputs = [ pkgs.ndg ];
  } ''
    mkdir -p $out
    ndg html \
      --input-dir ${./docs} \
      --output-dir $out \
      --title "My Project Documentation" \
      --module-options ${optionsJSON}/share/doc/nixos/options.json \
      --jobs $NIX_BUILD_CORES
  '';
in
  docs
```

This approach correctly handles the generation of options documentation using
`nixosOptionsDoc` and properly passes the result to ndg.

## Pro Tips & Best Practices

### Performance Optimization

- **Use multi-threading** for large documentation sets:

  ```bash
  ndg html -i ./docs -o ./html -T "My Project" -p $(nproc)
  ```

- **Selectively disable features** you don't need:

  ```bash
  # Omit flags to disable features (disabled by default if not provided)
  ndg html -i ./docs -o ./html -T "My Project"  # Search and highlighting disabled

  # Enable via CLI flags
  ndg html -i ./docs -o ./html -T "My Project" --generate-search --highlight-code

  # Or enable in config file (CLI flags override config)
  # ndg.toml:
  # generate_search = true
  # highlight_code = true
  ```

### Styling Tips

- **Use SCSS for complex stylesheets**: ndg automatically compiles SCSS files,
  allowing you to use variables, nesting, and mixins.

- **Start with exported templates**: Always begin customization by exporting the
  default templates:

  ```bash
  # Export to get the latest defaults
  ndg export --output-dir my-custom-templates
  ```

- **Customize specific pages**: When using a template directory, you can have
  different templates for different page types:

```txt
templates/
  default.html    # For regular markdown pages
  options.html    # For the options page
  search.html     # For the search page
```

### Content Organization

- **Use consistent header levels**: ndg generates the table of contents based on
  heading levels. Start with `h1` (`#`) for the title, then use `h2` (`##`) for
  major sections.

- **Structure your options with depth in mind**: The `--options-depth` parameter
  controls how the options TOC is generated. Organize your options to create a
  logical hierarchy.

- **Create an index.md file**: Place an `index.md` file in your input directory
  to serve as the landing page.

### GitHub Integration

- **Link to source with revision**: Use the `--revision` option to specify a
  GitHub revision for source links:

  ```bash
  ndg html -i ./docs -o ./html -T "My Project" --revision "main"
  ```

### Configuration Management

- **Create per-project config files**: For projects with complex documentation,
  create a dedicated `ndg.toml` file:

  ```toml
  input_dir = "docs"
  output_dir = "site"
  title = "Project Documentation"
  footer = "© 2025 My Organization"
  jobs = 8
  generate_search = true
  highlight_code = true
  options_toc_depth = 2
  stylesheet = "assets/custom.scss"
  script = ["assets/main.js", "assets/search-enhancer.js"]
  module_options = "generated/options.json"
  ```

- **Generate shell completions** once and save them to your shell configuration:

  ```bash
  # Generate and install completions
  mkdir -p ~/.local/share/bash-completion/completions
  ndg generate -o /tmp/ndg-gen
  cp /tmp/ndg-gen/completions/ndg.bash ~/.local/share/bash-completion/completions/ndg
  ```

### Content Enhancement

- **Use admonitions for important content**:

  ```txt
  <!-- Admonitions are flexible. -->
  ::: {.warning}
  This is a critical warning that users should pay attention to!
  :::

  <!-- Make it inline-->
  ::: {.tip} Here's a helpful tip to make your life easier. :::
  ```

- **Create custom ID anchors** for important sections:

  ```markdown
  ## Installation {#installation}

  Refer to the [installation instructions](#installation) above.
  ```

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
