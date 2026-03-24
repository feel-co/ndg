# Frontmatter

NDG supports TOML frontmatter in Markdown documents, allowing you to set
per-page metadata and control rendering behaviour without touching your
site-wide configuration.

## Syntax

Frontmatter must appear at the very beginning of the file, delimited by `+++` on
its own line. The block is parsed as TOML and stripped from the document before
the Markdown processor sees it.

```toml
+++
title = "My Custom Page Title"
description = "A short description of this page."
author = "Jane Doe"
template = "custom"
toc = false

[extra]
version = "1.2.0"
status = "stable"
+++

# Regular markdown begins here
```

> [!IMPORTANT]
> The opening `+++` must be the very first bytes of the file; no blank lines or
> spaces before it. Malformed TOML inside a valid `+++` block is logged as a
> warning and the block is ignored; the document is rendered as if no
> frontmatter were present.

## Supported Fields

### `title`

Overrides the page title extracted from the first H1 heading. Affects:

- The `<title>` element in the page `<head>`
- The `{{ title }}` template variable
- The `{{ page.title }}` template variable

```toml
+++
title = "Getting Started with NDG"
+++
```

If omitted, NDG falls back to the first H1 heading, then to the site-wide
`title` from your configuration.

### `description`

Per-page meta description. Injected as `<meta name="description">` and, when
OpenGraph is configured, as `og:description`. Takes precedence over any
`description` key set in the global `[meta.tags]` configuration for this page
only.

```toml
+++
description = "A step-by-step guide to installing and configuring NDG."
+++
```

### `author`

Per-page author. Injected as `<meta name="author">`. Overrides any `author` key
in the global `[meta.tags]` configuration for this page only.

```toml
+++
author = "Jane Doe"
+++
```

### `template`

Name of the template file to use for this page instead of the default template
resolution. The `.html` extension is optional.

```toml
+++
template = "landing"   # uses landing.html from your template directory
+++
```

Template resolution order when this field is set:

1. The named template file from your template directory (e.g. `landing.html`)
2. If the file is not found, falls back to the normal resolution: file-stem
   specific template, then `default.html`

### `toc`

Boolean. When set to `false`, suppresses the table of contents for this page.
Useful for pages where a TOC would be redundant or distracting, such as landing
pages or short reference pages.

```toml
+++
toc = false
+++
```

Defaults to `true` (TOC is generated whenever headings are present).

### `extra`

Arbitrary user-defined data, as a TOML table. The entire table is serialised and
made available in Tera templates as `{{ page.extra }}`. Individual keys are
accessible via `{{ page.extra.key }}`.

```toml
+++
[extra]
version = "1.2.0"
status = "stable"
badge = "https://example.com/badge.svg"
+++
```

In a custom template:

```html
{% if page.extra.version %}
<span class="version-badge">v{{ page.extra.version }}</span>
{% endif %} {% if page.extra.status == "stable" %}
<span class="badge stable">Stable</span>
{% endif %}
```

## Template Variables

All frontmatter fields are available in templates via the `page` object,
regardless of whether they affect rendering automatically:

<!--markdownlint-disable MD013-->

| Variable                 | Type            | Description                       |
| ------------------------ | --------------- | --------------------------------- |
| `{{ page.title }}`       | `String\|null`  | Frontmatter title, if set         |
| `{{ page.description }}` | `String\|null`  | Frontmatter description, if set   |
| `{{ page.author }}`      | `String\|null`  | Frontmatter author, if set        |
| `{{ page.template }}`    | `String\|null`  | Frontmatter template name, if set |
| `{{ page.toc }}`         | `Boolean\|null` | Frontmatter toc setting, if set   |
| `{{ page.extra }}`       | `Object\|null`  | Frontmatter extra table, if set   |

<!--markdownlint-enable MD013-->

Example template usage:

```html
{% if page.description %}
<meta name="description" content="{{ page.description }}" />
{% endif %} {% if page.extra %}
<div class="page-meta">
  {% if page.extra.status %}
  <span class="status">{{ page.extra.status }}</span>
  {% endif %}
</div>
{% endif %}
```

## Interaction with Global Configuration

Per-page frontmatter fields take precedence over their site-wide equivalents for
the affected page only. All other pages remain unaffected.

| Frontmatter field | Overrides                               |
| ----------------- | --------------------------------------- |
| `description`     | `meta.tags.description`                 |
| `author`          | `meta.tags.author`                      |
| `title`           | First H1 heading                        |
| `template`        | File-stem template, then `default.html` |
| `toc`             | Default TOC generation                  |

## Examples

### Landing page without a TOC

```toml
+++
title = "Welcome"
description = "NDG documentation home."
toc = false
+++

NDG is a fast documentation generator for Nix-adjacent projects...
```

### Page with a custom template and metadata

```toml
+++
title = "API Reference"
description = "Complete API reference for the ndg-commonmark library."
template = "reference"
author = "feel-co"

[extra]
version = "2.6.0"
+++
```

### Passing arbitrary data to a custom template

```toml
+++
[extra]
callout = "This page documents experimental features."
gh_issue = 42
+++
```

In your template:

```html
{% if page.extra.callout %}
<div class="callout warning">{{ page.extra.callout }}</div>
{% endif %}
```
