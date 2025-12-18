# Postprocessing

<!-- TODO: remove when postprocessing API is stable -->

NDG supports postprocessing of generated HTML, CSS, and JavaScript files to
optimize output size and performance. Postprocessing is optional and disabled by
default, however, testing and bug reports are welcome :)

## Configuration

Postprocessing behaviour of NDG is configured either through the `[postprocess]`
section in `ndg.toml`, or via CLI flags. To configure it in your `ndg.toml`:

```toml
[postprocess]
minify_html = true
minify_css = true
minify_js = true

[postprocess.html]
remove_comments = true
```

Or, alternatively, by using CLI overrides:

```bash
# Equivalent of setting `minify_html = true` under `[postprocess]`
# as seen in the configuration example above.
$ ndg html --config postprocess.minify_html=true
```

## Options

### Top-Level Options

| Option        | Type    | Default | Description                    |
| ------------- | ------- | ------- | ------------------------------ |
| `minify_html` | boolean | `false` | Enable HTML minification       |
| `minify_css`  | boolean | `false` | Enable CSS minification        |
| `minify_js`   | boolean | `false` | Enable JavaScript minification |

### HTML Options

| Option            | Type    | Default | Description                      |
| ----------------- | ------- | ------- | -------------------------------- |
| `remove_comments` | boolean | `true`  | Remove HTML comments from output |

## Minification

NDG supports minification of HTML, CSS and Javascript files for smaller
production documents at the cost of a tiny increase in generation time. In most
cases the cost is negligible, and the decrease of the document size is well
worth the penalty. However, this feature is completely optional.

If you are deploying your documentation to production, and if bandwith and file
size matters you will almost certainly want to enable minification. This is a
low-risk, high-reward feature that will likely yield faster page loads. You
will, however, want to disable minification if you are developing locally and
need readable source files. Additionally, you may wish to avoid minification if
you are aiming for the absolutely fastest document generation.

### HTML Minification

Uses `minify-html` for fast, spec-compliant HTML minification. While enabled,
NDG:

- Removes unnecessary whitespace while preserving content
- Respects whitespace-sensitive elements (`<pre>`, `<code>`, etc.)
- Removes HTML comments (configurable)
- Removes optional closing tags where safe
- Collapses boolean attributes

For example, for a given config:

```toml
[postprocess]
minify_html = true

[postprocess.html]
remove_comments = false  # keep HTML comments
```

and the input:

```html
<!DOCTYPE html>
<html>
  <body>
    <!-- Navigation -->
    <nav>
      <a href="/">Home</a>
    </nav>
  </body>
</html>
```

You will receive the following output:

```html
<!DOCTYPE html>
<html><body><nav><a href="/">Home</a></nav></body></html>
```

### CSS Minification

We use the `lightningcss` crate pulled by the `minify-html` crate for
production-grade CSS minification with parsing and optimization. This primarily
affects large stylesheets, such as the ones provided by the default NDG
template. While enabled, NDG:

- Removes unnecessary whitespace and comments
- Optimizes color values (`#ff0000` -> `red`)
- Merges duplicate rules
- Removes unused prefixes
- Validates CSS syntax during processing

For example, for a given config:

```toml
[postprocess]
minify_css = true
```

and the input:

```css
body {
  margin: 0;
  padding: 0;
  color: #ff0000;
}

/* Comment */
.container {
  max-width: 1200px;
}
```

You will receive the following output:

```css
body {
  margin: 0;
  padding: 0;
  color: red;
}
.container {
  max-width: 1200px;
}
```

### JavaScript Minification

We rely on the relatively new, but battle-tested `oxc_minifier` crate for
production-grade JavaScript minification with comprehensive optimizations and
dead code elimination. Besides speed, it boasts 100% correctness. While enabled,
NDG:

- Removes whitespace and comments
- Folds Constants(`1 + 2` -> `3`)
- Eliminates dead code (removes unused functions)
- Optimizes expressions (comma operators, etc.)
- Validates syntax during processing

For example, for a given config:

```toml
[postprocess]
minify_js = true
```

and the input:

```javascript
function greet(name) {
  // Say hello
  console.log("Hello, " + name);
  return true;
}
```

You will receive the following output:

```javascript
function greet(name) {
  console.log("Hello, " + name);
  return true;
}
```

### Complete Example

```toml
# ndg.toml
input_dir = "docs"
output_dir = "build"

[postprocess]
minify_html = true
minify_css = true
minify_js = true

[postprocess.html]
remove_comments = true
```

### Performance Impact

Postprocessing adds overhead to the build process but reduces output size:

| File Type  | Typical Reduction | Build Time Impact |
| ---------- | ----------------- | ----------------- |
| HTML       | 10-20%            | Minimal           |
| CSS        | 20-40%            | Low               |
| JavaScript | 15-30%            | Low               |

For most documentation sites, the build time impact is negligible (< 1 second
for typical projects).

### Error Handling

If minification fails (e.g., invalid CSS or JavaScript syntax), NDG will report
the error and stop the build. This helps catch syntax errors early:

```bash
Error: Failed to parse CSS: Unexpected token at line 5
```

To debug minification errors:

1. Disable minification for the failing file type
2. Fix the syntax error
3. Re-enable minification

## Extensibility

The postprocessing pipeline is quite new, but it is designed to be very
extensible. Each file type has detailed processor function with its own
configuration options. This design is obviously not for fun, as future
postprocessing capabilities are planned. Though, do not expect anything crazy. I
find postprocessing to be a little hacky, and it is only a last resort after
actual feature implementations.
