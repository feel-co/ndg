# NDG Syntax Reference

<!--toc:start-->

- [NDG Syntax Reference](#ndg-syntax-reference)
  - [Basic Markdown](#basic-markdown)
  - [Extended Syntax](#extended-syntax)
    - [Admonitions](#admonitions)
    - [Text Roles](#text-roles)
    - [Command and REPL Prompts](#command-and-repl-prompts)
    - [GitHub Callouts](#github-callouts)
    - [Headers with Anchors](#headers-with-anchors)
    - [Inline Anchors](#inline-anchors)
    - [List Items with Anchors](#list-items-with-anchors)
    - [Figures](#figures)
    - [Definition Lists](#definition-lists)
    - [File Includes](#file-includes)
    - [Autolinks](#autolinks)
    - [Option References](#option-references)
  - [NixOS Module Options](#nixos-module-options)
  - [Manpage Formatting](#manpage-formatting)

<!--toc:end-->

This document details all the syntax features supported by ndg and covers all
the special markup features that ndg supports beyond standard Markdown.

By using these extensions, you can create rich, semantically meaningful
documentation for your Nix projects that works well both as HTML and manpages.

> [!NOTE]
> While NDG is very clearly inspired by nixos-render-docs and nixpkgs flavored
> commonmark, 1:1 feature parity is not a concern. Most syntax features you
> might be familiar with _are_ supported, but there will be disparities. Please
> feel free to request new features that you think should be supported.

## Basic Markdown

ndg supports all standard CommonMark syntax including:

- Headings H1 to H6 (`#` to `######`)
- Emphasis (_italic_ and **bold**)
- Lists (ordered and unordered)
- Links and images
- Code blocks and inline code
- Blockquotes
- Horizontal rules
- Tables
- Footnotes

Additionally, ndg includes support for GitHub Flavored Markdown extensions like
strikethrough and task lists.

## Extended Syntax

Beyond standard Markdown, ndg provides several additional syntax features
designed to enhance documentation, particularly for Nix-related projects.

### Admonitions

Admonitions are colored callout boxes that highlight important information. They
use a fenced block syntax with a class indicator.

<!-- markdownlint-disable MD040 -->

```markdown
::: {.note} This is a note admonition. :::

::: {.warning} This is a warning admonition. :::

::: {.tip} This is a tip admonition. :::

::: {.important} This is an important admonition. :::

::: {.caution} This is a caution admonition. :::

::: {.danger} This is a danger admonition. :::

::: {.info} This is an info admonition. :::
```

You can also add an ID to an admonition for direct linking:

```
::: {.note #custom-id}
This note has a custom ID and can be linked to with
[link](#custom-id).
:::
```

### Text Roles

Text roles provide semantic markup for specific types of content. They use a
special syntax with curly braces and backticks.

```
{command}`ls -la`                    # Terminal command
{file}`/etc/nixos/configuration.nix` # File path
{option}`services.nginx.enable`      # NixOS option
{env}`HOME`                          # Environmentvariable
{var}`myVariable`                    # Nix variable
{manpage}`man(1)`                    # Man page reference
```

<!-- markdownlint-enable MD040 -->

Each role applies specific styling in the HTML output, making it easier to
distinguish different types of technical content.

> [!TIP]
> `{var}`, a.k.a "nix variables" are added in NDG to allow distinguishing
> between environment variables and other variables, e.g, in a config file.
> While it is called a "Nix variable", that is _only_ internal naming and the
> feature is _not_ specific to Nix variables.

### Command and REPL Prompts

Terminal commands with prompts can be typed with a shorthand syntax:

```markdown
`$ echo "Hello World"`
```

This renders as a command prompt with styling.

Similarly, for Nix REPL prompts:

```markdown
`nix-repl> 1 + 2`
```

### GitHub Callouts

ndg supports GitHub-style callouts:

```markdown
> [!NOTE]
> This is a note callout using GitHub's syntax.

> [!WARNING]
> This is a warning callout.

> [!TIP]
> This is a tip callout.

> [!IMPORTANT]
> This is an important callout.

> [!CAUTION]
> This is a caution callout.

> [!DANGER]
> This is a danger callout.
```

These are converted to the same styled admonition blocks as the fenced
admonition syntax.

### Headers with Anchors

You can add custom anchor IDs to headers for more reliable linking:

```markdown
## Installation {#installation}

Refer to the [installation section](#installation).
```

This creates a header with a specific ID that won't change even if you rename
the section.

### Inline Anchors

You can create anchor points anywhere in your document:

```markdown
[]{#my-anchor}

... some content ...

[Link to the anchor](#my-anchor)
```

This adds an invisible anchor that you can link to from elsewhere in the
document.

### List Items with Anchors

Add anchors to specific list items:

```markdown
- []{#item1} First item
- []{#item2} Second item

See the [second item](#item2).
```

### Figures

Create figures with captions:

```markdown
::: {.figure #sample-figure}

# Sample Figure Caption

![Alt text for image](path/to/image.png) :::
```

This creates a figure with proper semantic HTML including a caption.

### Definition Lists

Create definition lists with a simple syntax:

```markdown
Term : Definition for the term.

Another term : Definition for another term.
```

### File Includes

Include the contents of other files in your documentation:

````markdown
```{=include=}
path/to/file1.md
path/to/file2.md
```
````

This is useful for creating modular documentation from multiple smaller files.

### Autolinks

URLs are automatically converted to clickable links even without explicit
Markdown link syntax:

```markdown
Visit https://nixos.org for more information.
```

### Option References

References to NixOS options are automatically linked to the options
documentation page:

```markdown
Configure `services.nginx.enable` to start the web server.
```

When you have included a module options JSON file in your build, option
references like this will be automatically linked to the corresponding entry in
the options page. [^1]

[^1]: Work in progress, might not always work and may be removed in the future.

## NixOS Module Options

ndg can generate documentation for NixOS module options when provided with an
options JSON file (typically created with `nixosOptionsDoc`). Options are
automatically rendered with:

- Type information
- Default values
- Examples (if available)
- Descriptions
- Declaration locations (with links to source if available)

To include options documentation:

```bash
$ ndg html --module-options ./options.json ...
# => This will generate the document with an options.html page, also linked in
# the navbar by default
```

Options are presented in a hierarchical structure for easier navigation, with
the depth of parent categories controlled by the `--options-depth` parameter.

## Manpage Formatting

When generating manpages with ndg, special formatting is applied to convert
Markdown elements to troff syntax. The following elements receive special
treatment:

- Section headings become proper manpage section headings
- Code blocks are properly formatted with fixed-width text
- Lists are formatted with appropriate indentation
- Command prompts and REPL prompts are specially formatted
- Admonitions become indented blocks with appropriate headings
- Text roles are converted to bold or italic text as appropriate
- Links show both the link text and URL

To generate manpages from options:

```bash
$ ndg manpage --module-options ./options.json --output ./man/myproject.5 \
  --title "My Project" --section 5
```

---

Please refer to manpages and the README for direct command usage.
