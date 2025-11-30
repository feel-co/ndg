# Templating Guide

[Tera]: https://crates.io/crates/tera

NDG supports a flexible templating system powered by [Tera] and backed by our
own templating logic. Using this system, you may customize the appearance and
structure of your generated documentation while avoiding what could be referred
to as a "vendor lock-in".

## Template Types

There are several "special" templates that is expected during documentation
generation. NDG bundles its own templates that will be used by default, but you
may override them as you wish to make your documentation site truly unique.

### Core Templates

- **`default.html`** - Main template for documentation pages
- **`options.html`** - Template for NixOS module options page
- **`search.html`** - Template for the search page
- **`navbar.html`** - Navigation bar template (shared across all pages)
- **`footer.html`** - Footer template (shared across all pages)
- **`options_toc.html`** - Table of contents template for options page

### Assets

- **`default.css`** - Default stylesheet
- **`main.js`** - Scripts loaded for interactivity
- **`search.js`** - Secondary script used for the search functionality

## Using Custom Templates

As mentioned before, you may

### Template Directory

Using the template directory is the recommended way of overriding default
templates of NDG. In this method, you must use a template directory containing
all your custom templates:

```bash
ndg html --template-dir ./my-templates
```

Or in your `ndg.toml`:

```toml
template_dir = "./my-templates"
```

> [!TIP]
> The `ndg export-templates` command can be used to create a directory with the
> default templates, meant to be overridden by the user. ndg will prioritize
> templates provided by the user wherever provided. Missing page templates will
> cause ndg to use the default template for that page variant. E.g., providing a
> template directory with just `default.html` would use the `default.html`
> provided by the user, but the built-in templates for rest of the templatable
> components.

### Individual Template File

A dirty legacy method of quickly overriding `default.html` exists as
`--template`. This allows providing a quick override for **only the default
template**, which may be useful if you only want to override styles for
arbitrary generated pages, and not special pages or components such as the
navbar.

```bash
ndg html --template ./my-custom-template.html
```

This will use `my-custom-template.html` as the default.html template.

## Per-File Templates

You can create file-specific templates that will be used instead of
`default.html` for matching markdown files.

**Example:**

- `foo.md` -> will use `foo.html` template if it exists, otherwise falls back to
  `default.html`
- `about.md` -> will use `about.html` template if it exists, otherwise falls
  back to `default.html`

This is useful when you want different layouts for different sections of your
documentation. It can also be used for, e.g., `404.html` to generate your own
404 page with Markdown.

## Template Components

### Navbar Template (`navbar.html`)

Customize the navigation bar that appears in the header of all pages.

**Available Variables:**

- `{{ has_options }}` - HTML attributes for options link styling (e.g.,
  `class="active"`)
- `{{ generate_search }}` - Boolean indicating if search is enabled
- `{{ options_path }}` - Relative path to options page
- `{{ search_path }}` - Relative path to search page

#### Example

```html
<nav class="header-nav">
  <ul>
    <li {{ has_options|safe }}>
      <a href="{{ options_path }}">Options</a>
    </li>
    {% if generate_search %}
    <li><a href="{{ search_path }}">Search</a></li>
    {% endif %}
    <li><a href="/about.html">About</a></li>
  </ul>
</nav>
```

### Footer Template (`footer.html`)

Customize the footer that appears at the bottom of all pages.

**Available Variables:**

- `{{ footer_text }}` - Footer text from configuration

#### Example

```html
<footer>
  <p>{{ footer_text }}</p>
  <p>Built with <a href="https://github.com/feel-co/ndg">ndg</a></p>
</footer>
```

## Exporting Default Templates

To export all default templates for customization:

```bash
ndg export-templates --output ./templates
```

This will create a directory with all default templates that you can then
modify.

Use `--force` to overwrite existing files:

```bash
ndg export-templates --output ./templates --force
```

## Template Variables

### Common Variables (Available in all templates)

- `{{ title }}` - Page title
- `{{ site_title }}` - Site title from configuration
- `{{ footer_text }}` - Footer text from configuration
- `{{ content|safe }}` - Rendered markdown content
- `{{ toc|safe }}` - Table of contents HTML
- `{{ doc_nav|safe }}` - Document navigation HTML
- `{{ navbar_html|safe }}` - Rendered navbar HTML
- `{{ footer_html|safe }}` - Rendered footer HTML
- `{{ has_options }}` - Options link styling attributes
- `{{ generate_search }}` - Boolean for search feature
- `{{ custom_scripts|safe }}` - Custom scripts HTML
- `{{ meta_tags_html|safe }}` - Meta tags HTML
- `{{ opengraph_html|safe }}` - OpenGraph tags HTML

### Asset Paths

- `{{ stylesheet_path }}` - Path to stylesheet
- `{{ main_js_path }}` - Path to main JavaScript file
- `{{ search_js_path }}` - Path to search JavaScript file
- `{{ index_path }}` - Path to index page
- `{{ options_path }}` - Path to options page
- `{{ search_path }}` - Path to search page

### Options Page Specific

- `{{ heading }}` - Page heading
- `{{ options|safe }}` - Rendered options HTML

## Template Syntax

[Tera's documentation]: https://keats.github.io/tera/docs/#templates

Templates use the [Tera] templating engine, which is similar to Jinja2/Django
templates. Below are some examples provided for your convenience. Please refer
to the [Tera's documentation] for more details. Everything supported by Tera is
supported in NDG's templated documents.

### Variables

```html
{{ variable_name }}
```

Use the `safe` filter to render HTML without escaping:

```html
{{ html_content|safe }}
```

### Conditionals

```html
{% if generate_search %}
<div class="search">...</div>
{% endif %}
```

### Loops

```html
{% for item in items %}
<li>{{ item }}</li>
{% endfor %}
```

## Example: Custom Theme

Here's an example of creating a custom theme:

1. Export the default templates:

   ```bash
   ndg export-templates --output ./my-theme
   ```

2. Customize the templates in `./my-theme/`

3. Modify `navbar.html` to add custom links:

   ```html
   <nav class="header-nav">
     <ul>
       <li><a href="/docs.html">Docs</a></li>
       <li {{ has_options|safe }}>
         <a href="{{ options_path }}">Options</a>
       </li>
       <li><a href="/blog.html">Blog</a></li>
       {% if generate_search %}
       <li><a href="{{ search_path }}">Search</a></li>
       {% endif %}
     </ul>
   </nav>
   ```

4. Update `footer.html`:

   ```html
   <footer>
     <div class="footer-content">
       <p>{{ footer_text }}</p>
       <p>
         © 2025 My Project | <a href="/privacy.html">Privacy</a> | <a
         >href="/terms.html">Terms</a>
       </p>
     </div>
   </footer>
   ```

5. Create a custom template for your homepage (`index.html`):

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>{{ title }}</title>
    <link rel="stylesheet" href="{{ stylesheet_path }}" />
  </head>
  <body>
    <header>
      <h1 class="site-title">
        <a href="{{ index_path }}">{{ site_title }}</a>
      </h1>
      {{ navbar_html|safe }}
    </header>

    <main class="hero">
      <h1>Welcome to My Documentation</h1>
      {{ content|safe }}
    </main>

    {{ footer_html|safe }}
  </body>
</html>
```

Now you may use the custom theme freely using the `--template-dir` option.

```bash
ndg option --template-dir ./my-theme
```

## Configuration Example

In `ndg.toml`:

```toml
title = "My Awesome Site"
footer_text = "Built with ☕ and ♥️ "
template_dir = "./templates"
stylesheet_paths = ["./assets/custom.css"]
script_paths = ["./assets/custom.js"]

[opengraph]
"og:title" = "My Documentation"
"og:description" = "Comprehensive documentation for my project"
"og:image" = "./assets/banner.png"

[meta_tags]
description = "My amazing documentation"
keywords = "docs, markdown, documentation"
```

## Best Practices

For a clean(er) experience, we recommend that you:

1. **Start with defaults**: Export the default templates and modify them rather
   than starting from scratch
2. **Keep it modular**: Use the navbar and footer templates to keep common
   elements consistent
3. **Preserve CSS classes**: Keep the default CSS classes to maintain
   functionality (sidebar toggle, search, etc.)

The nature of a mixed-approach that NDG supports may sometimes yield results
that can be best described as annoying. Though it is perfectly possible to have
a good design _and_ keep your sanity.

## Troubleshooting

### Template not found

Make sure your template directory contains the required templates or that
fallback templates are available.

### Variables not rendering

- Check variable names match the documentation
- Use `{{ variable|safe }}` for HTML content
- Check for typos in variable names

### Styling issues

- Ensure your custom CSS doesn't conflict with default styles
- Check that CSS classes used by JavaScript features are preserved
- Test responsive design on mobile devices

### Per-file template not working

- Ensure the template filename matches the markdown filename (e.g., `foo.html`
  for `foo.md`)
- Check that the template is in the configured template directory
- Verify the template has valid [Tera] syntax
