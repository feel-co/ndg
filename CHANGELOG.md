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

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

This is the NDG changelog, aiming to describe changes that occurred within the
codebase to the extent that concerns _both users and contributors alike_. As
`feel-co/ndg` is a repository containing more than one project--i.e., a
monorepo--this document tracks all notable changes regardless of the affected
project and the scope.

[feel-co/ndg]: https://github.com/feel-co/ndg

[feel-co/ndg] strictly adheres to [Semantic Versioning], and the changelog
format below is a streamlined variant of the [Keep a Changelog] spec. Notable
changes are that all sections except for `Changed`, `Added`, `Removed` and
`Fixed` have been ignored.

## [Unreleased]

### Changed

- Streamlined codeblock tracking in `ndg-commonmark` processor
- Split utils module in `ndg-commonmark` for better organization

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

[Unreleased]: https://github.com/feel-co/ndg/compare/v2.4.0...HEAD
[2.4.0]: https://github.com/feel-co/ndg/compare/v2.3.2...v2.4.0
