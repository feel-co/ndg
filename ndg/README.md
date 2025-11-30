<!-- markdownlint-disable MD013 MD033 MD041 -->
<div id="doc-begin" align="center">
  <h1 id="header">
    <pre>NDG: Not a Docs Generator</pre>
  </h1>
  <br/>
  <a href="#what-is-it">Synopsis</a><br/>
  <a href="#features">Features</a> | <a href="#usage">Usage</a><br/>
  <a href="#pro-tips--best-practices">Best Practices</a>
  <br/>
</div>

## What is it?

[CommonMark]: https://commonmark.org/

**NDG** is a fast, customizable and convenient utility for generating HTML
documents and manpages for your Nix related projects from Markdown inputs, and
Nix module options. It can be considered a more opinionated and more specialized
alternative to MdBook, choosing to focus on Nix-related projects. While this
project is designed for [Hjem](https://hjem.feel-co.org) in mind, it is
perfectly possible to use NDG outside of Nix contexts. All Nix-specific
features, such as module options page and Nixpkgs-flavored [CommonMark] parsing
are optional and can be controlled via various knobs.

### Features

[ndg-commonmark]: https://crates.io/crates/ndg-commonmark

We would like for NDG to be fast, but also flexible and powerful. It boasts
several features such as

- **Markdown to HTML and Manpage conversion** with support for Nixpkgs-flavored
  CommonMark features[^1].
  - **Useful Markdown Companions**
    - **Automatic table of contents** generation from document headings
    - **Title Anchors** are automatically generated for headings
    - **GFM extensions** powered by [ndg-commonmark]
- **Search functionality** to quickly find content across documents
- **Nix module options support** to generate documentation from `options.json`
  - **Flexible options sidebar customization** to control visibility, naming,
    ordering, and depth of option categories
- **Fully customizable templates** to match your project's style fully
- **Incredibly fast** documentation generation for all scenarios
  - **Multi-threading support** for fast generation of large documentation sets

[^1]: This is a best-effort. Regular CommonMark is fully supported, but Nixpkgs
    additions may sometimes break due to the lack of a clear specification.
    Please open an issue if this is the case for you.

## Usage

The primary interface of NDG is the `ndg` CLI utility. There are several
subcommands to handle different documentation tasks.

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

You can review `ndg html` and `ndg manpage` commands' help texts to get more
insight.

### Quickstart

The easiest and fastest way to get started with NDG is using the `init`
subcommand to generate a configuration file, and tweak it per your needs.

```bash
# Generate a default ndg.toml configuration file
$ ndg init

# Generate a JSON configuration file instead
$ ndg init --format json --output ndg.json

# Generate in a specific location
$ ndg init --output configs/ndg.toml
```

Upon running `ndg init`, you will receive a configuration file in one of JSON or
TOML formats depending on the format specified, with all available options and
their default values as well as explanatory comments. You can then edit this
file to customize your documentation generation process.

> [!TIP]
> The generated configuration includes detailed comments for each option, making
> it easy to understand what each setting does without needing to refer to the
> documentation.

Once you have a configuration file, NDG will automatically detect it when ran in
the same directory. Though if you want to move the configuration file to a
nonstandard location, you may specify a path to the configuration file using the
`--config` option.

```bash
# Run with automatically detected config
$ ndg html

# Run with explicitly specified config
$ ndg html --config nested/path/to/ndg.toml
```

### Generating Your First Documents

To generate your documents, NDG requires one of two things: **Markdown Sources**
or **`options.json`**. Alternatively, for more comprehensive documents, you can
provide both. The latter will be used for an `options.html` with your module
options in the case Markdown sources are provided.

The Markdown sources are expected in an "inputs" directory, in which you can
dump your Markdown files with ndg-flavored Commonmark, and they will be
converted to HTML following the same structure. For example:

```plaintext
docs/
├── inputs/
│   ├── extending.md
│   └── index.md
└── package.nix
```

Assuming your input directory (`--input-dir`) is `inputs/`, you will receive the
following output:

```plaintext
docs-output/
├── assets/
│   ├── main.js
│   ├── search-data.json
│   ├── search.js
│   └── style.css
├── extending.html
├── index.html
├── options.html
└── search.html
```

You will notice the `index.html` and `options.html` documents representing
`index.md` and `options.md` from the inputs directory. The assets directory and
the `search.html` document are special outputs generated by NDG. We'll talk
about them later in the [templates section](#template-customization-made-easy).

This is the operating principle of NDG: you give it some Markdown inputs, and
you receive some HTML documents. It is all structured, so you can worry less
about site design and more about writing documentation. You _do_ write
documentation, right?

We'll continue talking about HTML generation in
[the relevant section](#generating-html-documents). For now, let's take a look
at how to configure NDG to fit your needs.

## Detailed Usage

### CLI Flags and Configuration Options

The CLI flags are the alternative method of configuring NDG, and can be used in
conjunction with the configuration file. While used together, the CLI flags can
be used to provide ad-hoc overrides of specific NDG behaviour provided by the
configuration file, or the default configuration, such as page indexing for
search and code highlighting. Two example flags accepted by `ndg html` are:

- `--generate-search`: Enables search functionality (disabled by default if
  omitted).
- `--highlight-code`: Enables syntax highlighting for code blocks (disabled by
  default if omitted).

Example usage:

```bash
ndg html --input-dir ./docs --output-dir ./html --title "My Project" --generate-search
```

In this example we're telling NDG to look for Markdown sources in the `docs/`
directory using the `--input-dir` flag and to dump the result in `html/` using
the `--output-dir` flag. Additionally, the `--title` and `--generate-search`
flags ensure that we use the correct title in the site's navbar, and to generate
search data.

Those flags will **override** the relevant values in the ndg configuration,
e.g., `ndg.toml` if provided. If you do not need a full configuration, you may
simply use the flags above to generate a complete documentation site without
ever needing to create your configuration. Do note, however, that some
configuration options are ONLY available in `ndg.toml`, and will not be
available to override from the CLI.

#### Configuration Files

NDG accepts two distinct configuration formats, TOML and JSON, to allow setting
your options persistently. Options like `generate_search` can be set in the
config file, but CLI flags will override them _if provided_.

TOML is the "blessed" format for NDG, though JSON is fully supported thanks to
Serde. The default format for generated configuration files is TOML.

Example `ndg.toml`:

```toml
input_dir = "docs"
output_dir = "html"
title = "My Project"
generate_search = true  # Can be overridden by CLI
```

#### Interactions Between CLI and Config

> [!NOTE]
> CLI flags always take precedence over config file settings. For instance, if
> your config file has `generate_search = false`, but you run
> `ndg html --generate-search`, search will be enabled.

If neither CLI nor config specifies an option, ndg uses sensible defaults (e.g.,
`generate_search` defaults to `true` if omitted from both; CLI flags override
config when present). For a full list of CLI options, see the help output with
`ndg html --help`.

#### Configuration Reference

Below is a comprehensive list of configuration options available in `ndg.toml`:

**Basic Options:**

- `input_dir` - Path to directory containing Markdown files
- `output_dir` - Output directory for generated documentation (default:
  `"build"`)
- `title` - Documentation title (default: `"ndg documentation"`)
- `footer_text` - Footer text (default: `"Generated with ndg"`)
- `jobs` - Number of threads for parallel processing (default: auto-detected)

**Feature Toggles:**

- `generate_search` - Enable search functionality (default: `true`)
- `highlight_code` - Enable syntax highlighting (default: `true`)
- `generate_anchors` - Generate heading anchors (default: `true`)

**Advanced Options:**

- `template_path` - Path to custom template file
- `template_dir` - Directory containing custom templates
- `stylesheet_paths` - Array of custom stylesheet paths
- `script_paths` - Array of custom JavaScript file paths
- `assets_dir` - Directory containing additional assets to copy
- `module_options` - Path to `options.json` for NixOS module documentation
- `options_toc_depth` - Depth of option categories in TOC (default: `2`)
- `manpage_urls_path` - Path to manpage URL mappings JSON file
- `revision` - Git revision for source file links (default: `"local"`)
- `tab_style` - How to handle tabs in code blocks: `"none"`, `"warn"`, or
  `"normalize"` (default: `"none"`)

**Metadata Options:**

- `opengraph` - Table of OpenGraph meta tags (e.g.,
  `{ "og:title" = "My Docs", "og:image" = "..." }`)
- `meta_tags` - Table of additional HTML meta tags (e.g.,
  `{ description = "...", keywords = "..." }`)

Example configuration:

```toml
input_dir = "docs"
output_dir = "build"
title = "My Project Documentation"
footer_text = "© 2025 My Project"
jobs = 4

# Enable features
generate_search = true
highlight_code = true
generate_anchors = true

# Code block handling
tab_style = "normalize"  # converts tabs to spaces

# Custom assets
stylesheet_paths = ["custom.css"]
script_paths = ["analytics.js"]
assets_dir = "static"

# Git integration
revision = "main"

# SEO metadata
[opengraph]
"og:title" = "My Project Docs"
"og:description" = "Comprehensive documentation"
"og:image" = "https://example.com/og-image.png"

[meta_tags]
description = "Complete guide to My Project"
keywords = "documentation,nix,tutorial"
author = "My Name"
```

#### Advanced Configuration Details

**Parallel Processing (`jobs`):**

The `jobs` option controls how many threads NDG uses for processing files. By
default, NDG auto-detects the number of CPU cores. For large documentation sets,
this significantly speeds up generation:

```bash
# Use 8 threads
ndg html --jobs 8

# Or in config
jobs = 8
```

**Tab Handling (`tab_style`):**

Controls how hard tabs in code blocks are handled:

- `"none"` - Leave tabs as-is (default)
- `"warn"` - Log warnings when tabs are detected
- `"normalize"` - Convert tabs to spaces (4 spaces per tab)

This is useful for ensuring consistent code block rendering across browsers.

**Manpage URL Mappings (`manpage_urls_path`):**

Provides a JSON file mapping manpage references to URLs. This allows NDG to
automatically link manpage citations in your documentation:

```json
{
  "man(1)": "https://man7.org/linux/man-pages/man1/man.1.html",
  "bash(1)": "https://man7.org/linux/man-pages/man1/bash.1.html"
}
```

**Custom Assets (`assets_dir`, `stylesheet_paths`, `script_paths`):**

- `assets_dir` - Directory to copy wholesale into the output (images, fonts,
  etc.)
- `stylesheet_paths` - Additional CSS files to include (can specify multiple)
- `script_paths` - Additional JavaScript files to include (can specify multiple)

Example:

```toml
assets_dir = "static"  # Copies static/* to build/assets/*
stylesheet_paths = ["theme.css", "custom.css"]
script_paths = ["analytics.js", "search-enhance.js"]
```

**SEO and Social Media (`opengraph`, `meta_tags`):**

These options inject metadata into the HTML `<head>` for better SEO and social
media sharing:

```toml
[opengraph]
"og:title" = "My Documentation"
"og:type" = "website"
"og:url" = "https://docs.example.com"
"og:image" = "https://docs.example.com/preview.png"
"og:description" = "Comprehensive documentation for My Project"

[meta_tags]
description = "Learn everything about My Project"
keywords = "documentation,tutorial,guide,nix"
author = "Jane Developer"
robots = "index,follow"
```

### Generating HTML Documents

As of `v2.0.0`, NDG features a more compartmentalized architechture with a focus
on distinguishing between output formats more carefully. Prior to version 2.0.0,
HTML was the "default" output format, and the only "blessed" one.

In more recent versions of NDG, HTML support is fully mature and is available
under the `html` subcommand. The `html` subcommand includes the following
options:

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

As covered in the previous chapters, you must NDG an **input directory**
containing your markdown files (`--input-dir`), an **output directory**
(`--output-dir`) to put the created files, and a **title** (`--title`) for basic
functionality. Alternatively, those can all be read from a config file, passed
to ndg through the `--config` option.

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

NDG allows generating manual pages for your project using your `options.json`.
Due to the shortcomings of Troff, manpage generation is not as robust or mature
as HTML. Nevertheless, the `manpage` subcommand allows generating a manpage from
your options:

```bash
$ ndg manpage -j ./options.json --output-file ./man/project.5 --title "My Project" --section 5
# => This will generate ./man/project.5 as your manpage.
```

> [!NOTE]
> For manpages, you **must** provide an `options.json` using `--module-options`
> or `-j`. Since we do not convert arbitrary pages in the Manpages mode, this is
> necessary to generate the main page containing module options.

## Customization

NDG is an opinionated CLI utility due to the reasons that warranted its
existence, but it still aims to support various colorful customization options
to tailor the output to your needs, regardless of how immodest they may be.

The HTML output design is handcrafted for
[Hjem](https://github.com/feel-co/hjem) by default as a small internal template,
and while this _is_ suitable for a majority of use cases, it may not be for you.
If you would like to go above and beyond, or simply are not very keen on those
templates, you are encouraged to _export the default templates_ and iterate on
them as you wish.

NDG provides an `export` subcommand that exports the default templates into a
specified directory of your choosing. You may modify those templates as you
wish, and use them in document generation by passing the `--template-dir` flag
to `ndg html`.

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

> [!NOTE]
> The `--template` flag will ONLY override the default template, which is used
> in the generation of the converted Markdown pages. To modify search and
> options pages, you may want to export the templates in full and modify them
> per your needs. This is an escape hatch for simple tweaks to converted pages.

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

See the [templating documentation](./docs/TEMPLATING) for details about the
templating. If your use-case is not supported, feel free to request a feature!

### Sidebar Customization

NDG allows you to customize the sidebar navigation through flexible
configuration options. You can control item ordering, add numbering, and
customize titles using pattern-based matching:

```toml
[sidebar]
numbered = true
ordering = "custom"

[[sidebar.matches]]
path = "getting-started.md"
new_title = "🚀 Quick Start"
position = 1

[[sidebar.matches]]
path = "installation.md"
new_title = "📦 Installation"
position = 2
```

This enables features like:

- **Sequential numbering** of sidebar items
- **Custom ordering** (alphabetical, custom position-based, or filesystem)
- **Pattern-based title customization** using exact or regex matching
- **Flexible positioning** for specific documentation items

For documentation generated from Nix module options, you can also customize the
options sidebar to control visibility, naming, ordering, and depth of option
categories. This is particularly useful for large module systems like NixOS or
Home Manager configurations.

See the [sidebar configuration guide](./docs/SIDEBAR) for complete documentation
including examples, use cases, and best practices.

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

[writing your own package]: https://github.com/feel-co/hjem/blob/main/docs/package.nix

Besides the CLI, NDG provides a simple but handy interface to interact
_directly_ with your Nix modules. While we recommend [writing your own package]
to work with NDG, a very basic _builder_ is provided under the NDG repository to
wrap and configure NDG for you. This is done by the `ndg-builder` package,
available from our flake outputs.

### Using NDG with Nix

Installing NDG with Nix is the recommended way of generating documentation for
your Nix projects. There are several ways to use NDG with Nix:

#### Installing from the Flake

[npins]: https://github.com/andir/npins

The simplest and fastest way of using NDG is getting it directly from our Nix
flake. A `ndg` package is exported, and it can be consumed in one of two ways:

1. Use NDG using the Nix3 CLI:

   ```bash
   # Install to your profile
   $ nix profile install github:feel-co/ndg#ndg # do not add the ndg-builder package!

   # Or run without installing
   $ nix run github:feel-co/ndg -- html -i ./docs -o ./result -T "My Project"
   ```

2. Add NDG to your flake inputs, and use its packages in your project

We generally recommend fetching NDG using Nixpkgs fetchers or [npins]. Due to
the inefficiency of Nix Flakes, using NDG as a flake input causes it to be
fetched each time the input is updated even if NDG is not actually in use. Using
Nixpkgs fetchers and npins allows benefiting from laziness.

See how [Hjem](https://github.com/feel-co/hjem/blob/main/docs/package.nix) does
it for an example of utilizing Npins for this task.

#### Using ndg-builder

We provide a `ndg-builder` function, meant as an escape hatch when you are not
interested in customizing NDG yourself. This abstracts away some of NDG's
complexities at the cost of some performance overhead, and a more limited
interface. In the case your needs are modest, you may be interested in using
this option.

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
`nixosOptionsDoc` and properly passes the result to NDG.

## Pro Tips & Best Practices

### Performance Optimization

- **Use multi-threading** for large documentation sets:

  ```bash
  ndg html -i ./docs -o ./html -T "My Project" -p $(nproc)
  ```

- **Selectively disable features** that you don't need:

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

#### Special Files

NDG treats certain files specially to maintain clear navigation hierarchy and
ensure proper site structure. These files have behaviors that differ from
regular documentation pages.

> [!TIP]
> **What are "Special Files"**?
>
> The following files are considered "special" (case-insensitive):
>
> - **`index.md`**
> - **`README.md`**
>
> In the case neither special file, nor an `options.json` is provided; NDG will
> create a fallback page that links available documents in the project. You are
> encouraged to provide a README or an index page to avoid serving your users a
> mostly empty fallback page.

**Special file behaviors:**

1. **Landing page priority** (`index.md` only):
   - Preferred source for the site's `index.html` landing page
   - If no `index.md` exists, NDG falls back to using `options.html` or creates
     a fallback index listing all documents

2. **Sidebar rendering** (both `index.md` and `README.md`):
   - Always appear **first** in the sidebar navigation
   - **Excluded from numbering by default** when `numbered = true` in sidebar
     config
   - Can be included in numbering by setting `number_special_files = true`
   - Can have custom titles via sidebar pattern matching

> [!NOTE]
> **Why this design?** Special files represent **conceptual entry points**
> rather than sequential content:
>
> - They serve as landing pages or overviews outside the normal documentation
>   flow
> - Numbering them (e.g., "1. Home", "2. Getting Started") creates awkward
>   visual hierarchy
> - Keeping them unnumbered and first emphasizes their role as anchors

With sidebar numbering enabled (`number_special_files = false`, the default):

```plaintext
docs/
├── index.md
├── README.md
├── getting-started.md
├── installation.md
└── usage.md
```

Renders as:

```plaintext
index.md          (unnumbered, appears first)
README.md         (unnumbered, appears second)
1. Getting Started
2. Installation
3. Usage
```

With `number_special_files = true`:

```plaintext
1. index.md
2. README.md
3. Getting Started
4. Installation
5. Usage
```

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
  script = ["assets/main.js", "assets/search-enhancer.js", "assets/web-analytics.js"]
  module_options = "generated/options.json"
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

- **Create memorable custom ID anchors** for important sections:

  ```markdown
  ## Installation {#my-epic-installation}

  Refer to the [installation instructions](#my-epic-installation) above.
  ```

## Building from Source

NDG is a tool written in Rust, and a Nix package is available for easily
building and consuming it.

```bash
# Build from cloned directory
nix build .#ndg -Lv

# Install on your system profile, only viable on non-NixOS
nix profile add github:feel-co/ndg
```

In the case you're planning to hack on NDG, `.envrc` is provided for Direnv
integration and you may use `cargo build` inside a dev shell, e.g., by using
`nix develop`.

```bash
# Result will be available in target/debug
$ cargo build

# You may also build and test the documentation
$ cargo doc --workspace # target/doc
```

## License

<!-- markdownlint-disable MD059 -->

This project is made available under Mozilla Public License (MPL) version 2.0.
See [LICENSE](LICENSE) for more details on the exact conditions. An online copy
is provided [here](https://www.mozilla.org/en-US/MPL/2.0/).

<!-- markdownlint-enable MD059 -->

<div align="right">
  <a href="#doc-begin">Back to the Top</a>
  <br/>
</div>
