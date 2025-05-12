<h1 align="center">
  <br>
  NDG: Not a Docs Generator
  <br>
</h1>

## What is it?

**ndg** is a fast, customizable and convenient utility for generating HTML
documents and manpages for your Nix related projects from Markdown and
options you use in any set of modules. It can be considered a more opinionated
and more specialized MdBook, focusing on Nix-related projects. Though it can
also be used outside of Nix projects.

### Features

We would like for ndg to be fast, but also flexible and powerful. It boasts
several features such as

- **Markdown to HTML and Manpage conversion** with support for all[^1]
   Nixpkgs-flavored Commonmark features.
- **Useful Markdown Companions**
  - **Automatic table of contents** generation from document headings
  - **Title Anchors** - Automatically generates anchors for headings
- **Search functionality** to quickly find content across documents (can be
  disabled with `--generate-search/-S false`)
- **Nix module options support** to generate documentation from `options.json`
- **Fully customizable templates** to match your project's style fully
- **Multi-threading support** for fast generation of large documentation sets
- **Incredibly fast** documentation generation for all scenarios

[^1]: This is a best-effort. Regular commonmark is fully supported, but Nixpkgs
    additions may sometimes break due to the lack of a clear specification.
    Please open an issue if this is the case for you.

## Usage

ndg has several subcommands to handle different documentation tasks:

```
Usage: ndg [OPTIONS] [COMMAND]

Commands:
  init      Initialize a new NDG configuration file
  generate  Generate shell completions and manpages for ndg itself
  html      Generate HTML documentation from markdown files and/or module options
  manpage   Generate manpage from module options
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose               Enable verbose debug logging
  -c, --config <CONFIG_FILE>  Path to configuration file (TOML or JSON)
  -h, --help                  Print help
  -V, --version               Print version
```

### Getting Started with Configuration

The easiest way to get started with ndg is to use the `init` command to generate a default configuration file:

```bash
# Generate a default ndg.toml configuration file
ndg init

# Generate a JSON configuration file instead
ndg init --format json --output ndg.json

# Generate in a specific location
ndg init --output configs/ndg.toml
```

This will create a well-documented configuration file with all available options and their default values. You can then edit this file to customize your documentation generation process.

The generated configuration includes detailed comments for each option, making it easy to understand what each setting does without needing to refer to the documentation.

Once you have a configuration file, ndg will automatically detect it when run from the same directory, or you can specify it explicitly with the `--config` option:

```bash
# Run with automatically detected config
ndg html

# Run with explicitly specified config
ndg html --config path/to/ndg.toml
```

The first subcommand in ndg's compartmentalized architecture is for
HTML generation. The `html` subcommand includes the following options:

```
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
          Path to directory containing template files (default.html, options.html, etc.)
  -s, --stylesheet <STYLESHEET>
          Path to custom stylesheet
      --script <SCRIPT>
          Path to custom Javascript file to include. Can be specified multiple times to include multiple script files.
  -T, --title <TITLE>
          Title of the documentation
  -f, --footer <FOOTER>
          Footer text for the documentation
  -j, --module-options <MODULE_OPTIONS>
          Path to a JSON file containing module options
      --options-depth <OPTIONS_TOC_DEPTH>
          Depth of parent categories in options TOC
      --manpage-urls <MANPAGE_URLS>
          Path to manpage URL mappings JSON file
      --generate-search <GENERATE_SEARCH>
          Whether to generate search functionality
      --highlight-code <HIGHLIGHT_CODE>
          Whether to enable syntax highlighting for code blocks
      --revision <REVISION>
          GitHub revision for linking to source files
  -h, --help
          Print help
```

You must give ndg a **input directory** containing your markdown files
(`--input-dir`), an **output directory** (`--output-dir`) to put the created files, and
a **title** (`--title`) for basic functionality. Alternatively, those can all be
read from a config file, passed to ndg through the `--config` option.

For example:

```bash
ndg html -i ./ndg-example/docs -o ./ndg-example/html -T "Awesome Nix Project"
```

where `./ndg-example/docs` contains markdown files that you would like to
convert, and `./ndg-example/html` is the result directory.

The `manpage` subcommand allows generating a manpage from your options:

```bash
ndg manpage -j ./options.json -o ./man/project.5 -T "My Project" -s 5
```

The `generate` subcommand helps create shell completions and manpages for ndg itself:

```bash
ndg generate -o ./dist
```

Optionally you may pass an `options.json` to also render an `options.html` page
for a listing of your module options.

### Customization

ndg supports various customization options to tailor the output to your needs:

#### Custom Templates

You can provide your own HTML template using either the `--template` option for
a single template file:

```bash
ndg html -i ./docs -o ./html -T "Awesome Project" -t ./my-template.html
```

Or use the `--template-dir` option to provide a directory containing multiple
templates:

```bash
ndg html -i ./docs -o ./html -T "Awesome Project" --template-dir ./my-templates
```

When using a template directory, ndg will look for the following files:

- `default.html` - Main template for documentation pages
- `options.html` - Template for options page
- `search.html` - Template for search page
- `options_toc.html` - Template for options table of contents

The default template files are provided in `templates/`.

Custom templates use the Tera templating engine, so your templates should use
Tera syntax. Below are the variables that ndg will attempt to replace.

- `{{ title }}` - Document title
- `{{ content|safe }}` - Main content area (unescaped HTML)
- `{{ toc|safe }}` - Table of contents (unescaped HTML)
- `{{ doc_nav|safe }}` - Navigation links to other documents (unescaped HTML)
- `{{ has_options|safe }}` - Conditional display for options page link
- `{{ footer_text }}` - Footer text specified via the --footer option
- `{{ generate_search }}` - Boolean indicating whether search functionality is enabled
- `{{ stylesheet_path }}` - Path to the stylesheet
- `{{ script_paths }}` - List of paths to script files

Each template can use these variables as needed. For example, in the `default.html` template:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ title }}</title>
    <link rel="stylesheet" href="{{ stylesheet_path }}">
    {% for script_path in script_paths %}
    <script src="{{ script_path }}"></script>
    {% endfor %}
</head>
<body>
    <header>
        <h1>{{ title }}</h1>
        {% if generate_search %}
        <div class="search-container">
            <input type="text" id="search-input" placeholder="Search documentation...">
            <a href="search.html" class="search-button">Advanced Search</a>
        </div>
        {% endif %}
    </header>
    
    <div class="container">
        <nav class="sidebar">
            {{ toc|safe }}
            {{ doc_nav|safe }}
            {% if has_options %}
            <div class="options-link">
                <a href="options.html">Module Options</a>
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
</body>
</html>
```

#### Custom Stylesheet

You can provide your own CSS stylesheet using the `--stylesheet` option:

```bash
ndg html -i ./docs -o ./html -T "My Project" -s ./my-styles.css
```

ndg is able to compile SCSS as well, should you choose to pass it a file with
the `.scss` extension. The stylesheet will be compiled in runtime, and placed as
a `.css` file in the output directory.

#### NixOS Options

To include a page listing NixOS options, provide a path to your `options.json`
file:

```bash
ndg html -i ./docs -o ./html -T "My Project" -j ./options.json
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
ndg html -i ./docs -o ./html -T "My Project" --generate-search false
```

This will prevent the creation of search.html and the search index data, and the
search widget won't appear in the navigation.

## Nix Integration

### Using ndg with Nix

ndg can be integrated into Nix builds to generate documentation for your projects. Here are several ways to use ndg with Nix:

#### Installing from the Flake

The simplest way to install ndg is directly from its flake:

```bash
# Install to your profile
nix profile install github:feel-co/ndg

# Or run without installing
nix run github:feel-co/ndg -- html -i ./docs -o ./result -T "My Project"
```

#### Using ndg-builder

The recommended way to generate documentation in Nix builds is to use the included `ndg-builder` function, which properly handles all the complexities:

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
      packages.${system}.docs = ndg.packages.${system}.ndg-builder {
        title = "My Project Documentation";
        inputDir = ./docs;
        stylesheet = ./assets/style.css;
        scripts = [ ./assets/custom.js ];

        # For module options
        rawModules = [
          ./modules/options.nix
          {
            options.example.option = pkgs.lib.mkOption {
              type = pkgs.lib.types.str;
              default = "value";
              description = "An example option";
            };
          }
        ];

        # Optional: set the depth of options TOC
        optionsDepth = 3;
      };
  };
}
```

#### Manual Approach Using runCommandLocal

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

This approach correctly handles the generation of options documentation using `nixosOptionsDoc` and properly passes the result to ndg.

## Pro Tips & Best Practices

### Performance Optimization

- **Use multi-threading** for large documentation sets:
  ```bash
  ndg html -i ./docs -o ./html -T "My Project" -p $(nproc)
  ```

- **Selectively disable features** you don't need:
  ```bash
  # Disable search if you don't need it
  ndg html -i ./docs -o ./html -T "My Project" --generate-search false

  # Disable syntax highlighting for better performance
  ndg html -i ./docs -o ./html -T "My Project" --highlight-code false
  ```

### Styling Tips

- **Use SCSS for complex stylesheets**: ndg automatically compiles SCSS files, allowing you to use variables, nesting, and mixins.

- **Customize specific pages**: When using a template directory, you can have different templates for different page types:
  ```
  templates/
    default.html    # For regular markdown pages
    options.html    # For the options page
    search.html     # For the search page
  ```

### Content Organization

- **Use consistent header levels**: ndg generates the table of contents based on heading levels. Start with `h1` (`#`) for the title, then use `h2` (`##`) for major sections.

- **Structure your options with depth in mind**: The `--options-depth` parameter controls how the options TOC is generated. Organize your options to create a logical hierarchy.

- **Create an index.md file**: Place an `index.md` file in your input directory to serve as the landing page.

### GitHub Integration

- **Link to source with revision**: Use the `--revision` option to specify a GitHub revision for source links:
  ```bash
  ndg html -i ./docs -o ./html -T "My Project" --revision "main"
  ```

### Configuration Management

- **Create per-project config files**: For projects with complex documentation, create a dedicated `ndg.toml` file:
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
  ```markdown
  ::: {.warning}
  This is a critical warning that users should pay attention to!
  :::

  ::: {.tip}
  Here's a helpful tip to make your life easier.
  :::
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
