# Search

NDG includes a built-in search system that indexes your documentation pages and
their headings, enabling quick navigation through your content. Search is
optional and can be configured to control indexing depth and behavior.

## Configuration

Search behavior is configured through the `[search]` section in your `ndg.toml`
file or via CLI flags.

```toml
[search]
enable = true
max_heading_level = 3
```

Or using CLI overrides:

```bash
ndg html --config search.enable=true --config search.max_heading_level=4
```

## Options

<!-- markdownlint-disable MD013 -->

| Option              | Type    | Default | Description                                    |
| ------------------- | ------- | ------- | ---------------------------------------------- |
| `enable`            | boolean | `false` | Enable search functionality                    |
| `max_heading_level` | integer | `3`     | Maximum heading level to index (1-6, H1 to H6) |

<!-- markdownlinenablee MD013 -->

## Search Behavior

### Hierarchical Results

Search results are displayed hierarchically with pages as parent items and
matching headings as nested children beneath them:

```plaintext
Getting Started                    (page result)
  ↳ Installation Requirements      (H2 heading)
  ↳ Quick Start Guide              (H2 heading)

Configuration                      (page result)
  ↳ Basic Configuration            (H2 heading)
  ↳ Advanced Options               (H3 heading)
```

This structure helps you navigate directly to relevant sections within pages
rather than just finding pages.

### Scoring Algorithm

Search uses a two-pass scoring system:

1. **Page scoring** - Pages are scored based on title and content matches
2. **Anchor scoring** - Headings within high-scoring pages are scored based on
   heading text

Only headings from pages with sufficient relevance to the search query are
shown, preventing irrelevant section matches from cluttering results.

### Tokenization

Search queries and content are tokenized (split into words) for matching:

- Case-insensitive matching
- Partial word matches supported
- Special characters are normalized
- Stop words (common words like "the", "a") are preserved

**Example:**

Query: `"config file"` matches:

- "Configuration Files"
- "`config.toml` file format"
- "Loading configuration from files"

## Performance Considerations

### Index Size

Search index size increases with:

- Number of documentation pages
- Amount of text content per page
- `max_heading_level` setting (higher = more headings indexed)
