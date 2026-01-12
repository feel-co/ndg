<!-- markdownlint-disable no-duplicate-heading -->

# NDG Changelog

<!--
If you are a contributor looking at this document, then chances are that you're working
on a pull request. You are requested to add your changes under the "Unreleased" section
as tags will be created at the discretion of maintainers. If your changes fix an
existing bug, you must describe the new behaviour (ideally in comparison to the
old one) and put it under the "Fixed" subsection. Linking the relevant open
issue is not necessary, but good to have. Otherwise, general-purpose changes can
be put in the "Changed" section or, if it's just to remove code or
functionality, under the "Removed" section.
-->

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

This is the NDG changelog, aiming to describe changes that occurred within the
codebase to the extent that concerns _both users and contributors alike_. As
`feel-co/ndg` is a repository containing more than one project--i.e., a
monorepo--this document tracks all notable changes regardless of the affected
project and the scope.

NDG strictly adheres to [Semantic Versioning], and the changelog format below is
a streamlined variant of the [Keep a Changelog] spec. Notable changes are:

- All sections except for `Changed`, `Added`, `Removed` and `Fixed` have been
  ignored.
- Release date is not tracked

## [2.6.0]

### Added

- Collapsible sidebar sections ("Documents" and "Contents") with smooth
  open/close animation and localStorage state persistence. Toggle state syncs
  between desktop sidebar and mobile drawer.
- Right-side "On this page" table of contents that tracks scroll position. Shows
  on wide screens (>=1400px), hidden on mobile. Only appears on regular
  documentation pages, and not on options page.
- Auto-generated `id` attributes on all HTML headers (h1-h6) without explicit
  `{#custom-id}` syntax. Uses slugified text content, enabling scroll spy and
  anchor navigation to work consistently.
- Smooth scrolling behavior for anchor links (`scroll-behavior: smooth` on
  `html`)

### Fixed

- Lazy continuation in GFM callouts (lines without `>` that continue a callout)
  are now correctly included; callouts no longer terminate prematurely and lose
  content. This fixes broken/missing admonition content in generated HTML.
- ATX header detection in ndg-commonmark is now more robust; ATX headers (1–6
  `#`) are strictly checked to avoid false block starts. Fixes malformed string
  continuation in admonition rendering that previously produced literal `\\n` in
  HTML output.
- Fallback index page no longer displays inline anchor syntax literally (e.g.,
  `{#extending-hjem}`). The title extraction now properly strips `{#anchor-id}`
  patterns.
- `strip_markdown()` in ndg-commonmark now preserves inline code content. The
  function previously silently dropped inline code like `` `grep` `` because
  `NodeValue::Code` stores text in `.literal` rather than as child nodes.

## [2.5.1]

### Fixed

- Manpage generation now properly formats lists, escapes special characters, and
  strips HTML wrappers
  - List items render as proper `.IP` macros instead of literal text
  - Leading dots and apostrophes are correctly escaped to prevent troff command
    interpretation
  - Default/example values no longer include `<html>/<body>/<p>` tags
  - "Declared by" sections use correct troff bold/italic formatting
  - Fixed duplicate processor initialization (something around 30+ -> 1 as per
    my testing)

### Changed

- Performance optimizations in manpage generation
  - Replaced `get_roff_escapes()` function with static `ROFF_ESCAPES` HashMap
    (eliminates thousands of allocations)
  - Removed `ROLE_PATTERN`, `COMMAND_PROMPT`, `REPL_PROMPT`, and `INLINE_CODE`
    regexes

## [2.5.0]

### Added

- Support for multiple configuration files via repeatable `--config-file` flag
  - Config files are merged in order with later files overriding earlier ones
- New `--config KEY=VALUE` flag for fine-grained configuration overrides
  - Supports boolean (true/false/yes/no/1/0), string, path, and numeric types
  - Support for nested config keys with dot notation (e.g.,
    `sidebar.options.depth`)
- Sidebar configuration for documentation and options ordering
  - Pattern-based matching for custom sidebar grouping
  - Configurable ordering algorithms (alphabetical, custom)
  - Support for read-only option badges in sidebar
- Hierarchical search configuration via `search` section
  - `search.enable` replaces deprecated `generate_search` boolean
  - `search.max_heading_level` for controlling search depth
  - **Deprecated** `generate_search` in favor of `search.enable`
- Asset copying configuration
  - Fine-grained control over custom asset processing
  - Automatic minification of CSS/JS files during asset copying
  - CSS and JavaScript minification support
  - Configurable minification options for generated HTML assets
  - Per-file minification for custom assets in subdirectories
  - Support for multiple minification targets (CSS, JS, HTML)
- Language support expansion in ndg-commonmark
  - Added 11 new language parsers: `tsx`, `c_sharp`, `asm`, `diff`, `elixir`,
    `jsdoc`, `printf`, `regex`, `zig`, `markdown_inline`, and `php_only`

### Changed

- Renamed `--config` flag to `--config-file` for clarity
  - New `--config` flag now accepts KEY=VALUE override pairs
- Sidebar font size increased from 12px to 14px for better legibility
- Count badge colors changed from orange to primary purple for better contrast
- **Deprecated** `--options-depth` CLI flag in favor of config-based approach
  - Flag still works but emits deprecation warning
  - Use `--config sidebar.options.depth=N` or config file instead
- **Deprecated** `options_toc_depth` config key in favor of
  `sidebar.options.depth`
- **Deprecated** `meta_tags` and `opengraph` config fields in favor of nested
  `meta` configuration
  - Old: `meta_tags = { description = "..." }` and
    `opengraph = { "og:title" = "..." }`
  - New: `meta.tags = { description = "..." }` and
    `meta.opengraph = { "og:title" = "..." }`

  Example:

  ```toml
  [meta.tags]
  description = "My awesome documentation"
  keywords = "rust,nix,docs"

  [meta.opengraph]
  "og:title" = "My Project Docs"
  "og:image" = "/path/to/image.png"
  ```

  Or:

  ```toml
  [meta]
  opengraph = { "og:title" = "..." }
  tags = { description = "..." }
  ```
- **Deprecated** `generate_search` boolean in favor of `search.enable`
  - Old: `generate_search = true`
  - New: `search.enable = true` (or `search = { enable = true }`)
- Improved syntax highlighting performance in ndg-commonmark via resource reuse
  - Processor and renderer instances are now reused across highlight operations
  - Eliminated repeated initialization overhead
- Template path logic simplified to remove confusing behavior

### Fixed

- Config file values for `generate_search` and `highlight_code` are now
  respected when CLI flags are not provided
- Silent regex failures in sidebar matching now properly panic with clear error
  messages
- Regex compilation performance improved by caching compiled patterns during
  validation
- Included files are now properly filtered from search index
  - Files included via `{=include=}` directives no longer appear as standalone
    search results
  - Included content is indexed under the parent document with proper heading
    anchors
- Search result paths correctly resolve to included file parent documents
- Non-deterministic sidebar ordering for special files (index.html, 404.html)
  fixed
- Heading text extraction now correctly handles inline elements (links,
  emphasis) in ndg-commonmark

## [2.4.1]

### Added

- Regression test for HTML escaping in search functionality

### Fixed

- HTML escaping in search results to properly handle special characters in
  option keys

### Changed

- Replaced `unwrap()` calls with better error handling patterns using `expect()`
  - Improved error handling in file includes API for safer operation
- Streamlined codeblock tracking and split utils module for better organization
- Added logging when falling back to never matching regex patterns
- Improved error messages and documentation throughout codebase

## [2.4.0]

### Added

- Configurable tab handling for code blocks
- Optional `valid_options` to `MarkdownProcessor` for option validation
- Regression test for newline counts in codeblocks

### Changed

- Made `SearchIndex::new` a `const fn` for better performance
- Removed option auto-detection; now requires explicit input
- Removed `RefCell` for thread-safe `MarkdownProcessor`
- Cheap-clone syntax highlighting instance
- Consolidated duplicate JSON extraction functions
- Search performance optimizations:
  - Moved search dataset size checks before expensive scoring
  - Pre-compute `lowerSearchTerms` once before the doc loop in workers
  - Avoid mutating the original search array
  - Increment `doc_id` by `documents_count` +1
  - Allows users to retry loading search data if it initially fails
  - Validates data is a string before calling `toLowerCase()`
  - Partially implemented web-workers for search
- Improved whitespace handling in HTML generation
- Matched divider width for search widget
- Properly migrated away from legacy HTML "escaping" logic
- Documented tab normalization in API reference
- Made documentation more consistent; cleaned up wording
- More descriptive API docs; linked Github repository
- Included more examples in ndg-commonmark
- Documented example usage & compliance
- Refactored doc nav entry rendering into helper function
- Moved asset-related utilities to submodule
- Custom error types with colored traces using color_eyre
- Made help text shorter for HTML subcommand
- Normalized absolute paths in include-to references
- Made component colors darker in dark mode
- Skip files not generated as HTML in fallout index
- Encode text with `html-escape` in TOC generation
- Allow omitting boolean values for `generate_search` and `highlight_code`
- Switched to a safer DOM selector

### Fixed

- Fixed silent failure in syntax highlighting initialization (ndg-commonmark)
- Prevented panic in theme name fallback logic (ndg-commonmark)
- Prevented panic on empty fence character extraction (ndg-commonmark)

[Unreleased]: https://github.com/feel-co/ndg/compare/v2.6.0...HEAD
[2.6.0]: https://github.com/feel-co/ndg/compare/v2.5.1...v2.6.0
[2.5.0]: https://github.com/feel-co/ndg/compare/v2.4.1...v2.5.0
[2.5.1]: https://github.com/feel-co/ndg/compare/v2.5.0...v2.5.1
