# feel-co/ndg

This repository contains our in-house documentation tooling, designed to
accommodate our various documentation needs, and the relevant dependencies that
are needed to make it happen. The most critical component stored here, which you
might be the most interested in, is [NDG](./ndg); a fast, robust and
customizable documentation utility to generate a complete documentation website
for your Nix-adjacent projects on the fly. [ndg-commonmark](./ndg-commonmark) is
a Rust crate that handles parsing flavored CommonMark, which does not provide
any CLI features but appears as a _very_ powerful Nixpkgs-flavored CommonMark
parser library.

If you plan to make use of NDG, or the ndg-commonmark library, please read the
appropriate sections below. More advanced documentation can be found in each
crate's own directory. If you are here to contribute to one of those projects,
please see the [Contributing](#contributing) section.

## Repository Structure

[Github Flavored Markdown]: https://github.github.com/gfm/

```plaintext
.
├── crates
├── ndg
├── ndg-commonmark
└── nix
```

The NDG repository consists of three critical, and one auxiliary component. The
critical components that you might be interested in are:

- [`ndg`](./ndg) directory contains the source code for the NDG utility
- [`ndg-commonmark`](./ndg-commonmark) directory contains a Rust library crate
  that implements a parser for Nixpkgs-flavored CommonMark with our own
  additions as well as [Github Flavored Markdown] (GFM).
- [`nix`](./nix) directory contains our packaging utilities via Nix, which is
  the primary method of interacting with this repository.

The `crates/` directory contains the less important, often private crates that
are used in constructing the `ndg` and `ndg-commonmark` repositories, or helper
crates that assist in repository maintenance or packaging---such as `xtask`.

### ndg

[![Build Status](https://img.shields.io/github/actions/workflow/status/feel-co/ndg/rust.yml?branch=main)](https://github.com/feel-co/ndg/actions/workflows/rust.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.90+-orange.svg)](https://www.rust-lang.org)
[![GitHub Release](https://img.shields.io/github/v/release/feel-co/ndg)](https://github.com/feel-co/ndg/releases/latest)

[Hjem]: https://github.com/feel-co/hjem
[NVF]: https://github.com/notashelf/nvf
[quickstart document]: ./ndg/README.md
[documentation for this repository]: https://ndg.feel-co.org

> [!TIP]
> To begin working with NDG, take a look at the [quickstart document]. CLI
> usage, supported syntax, Nix integration and so on are generously documented
> in this document. Alternatively, the [documentation for this repository]
> dogfoods NDG to prepare a live rendered version of the project documentation.
> I

NDG; or, "Not A Docs Generator" is our in-house documentation utility for Nix
and Nix-adjacent projects, replacing most commonly used tooling such as MdBook
with a tool far less opinionated and Nix-friendly for our needs. That said, NDG
also aims to be more "batteries included" compared to existing and somewhat
popular Nixpkgs-first tooling (à la `nixos-render-docs`), and offers various
improvements compared to those tooling such as a **higher degree of
customization**, **more intuitive architecture**, **graceful error recovery**
and most critically_a lot_ of room for further extension as NDG is not tied to
Nixpkgs, and is not constrained by backwards compatibility with legacy baggage.

While the _main_ focus of this tool is to generate documentation for projects
that provide Nix module systems (such as [Hjem], or [NVF]), you may _easily_ use
it as a MdBook replacement in your projects. You may even disable Nix-specific
features completely and use it for _any_ project! NDG offers a flexible Markdown
parser, a robust templating system and various configuration options to allow
designing beautiful documentation for your projects. The only thing it _can't_
do is to write the documentation for you...

### ndg-commonmark

[![Documentation](https://img.shields.io/docsrs/ndg-commonmark/latest)](https://docs.rs/ndg-commonmark/latest/)
[![Rust Version](https://img.shields.io/badge/rust-1.90+-orange.svg)](https://www.rust-lang.org)
[![Crates.io Version](https://img.shields.io/crates/v/ndg-commonmark)](https://crates.io/crates/ndg-commonmark)

[published on crates.io]: https://crates.io/crates/ndg-commonmark
[documentation for latest tagged release]: https://docs.rs/ndg-commonmark/latest/
[documentation for the nightly variant]: https://ndg.feel-co.org/api

> [!IMPORTANT]
> ndg-commonmark is a public Rust library designed to allow interacting with
> Nixpkgs-flavored CommonMark. It is used by NDG under the hood to parse and
> render Nixpkgs-flavored CommonMark. If you are interested in working with
> ndg-commonmark in your projects take a look at the
> [documentation for latest tagged release] or
> [documentation for the nightly variant].

The CLI component of this repository, NDG, has began as a Pandoc wrapper that
preprocessed some of your options and structured them into a format more
suitable for Pandoc. This was fine, of course, but it required a lot of Lua
filter work and it could not deal with Nixpkgs flavored CommonMark---which is to
be expected, as Nixpkgs-flavored CommonMark is not formally codified, and lives
only as a reference document in the Nixpkgs repository.

NDG 1.0.0 was the Rust rewrite of this Pandoc wrapper, which saw us drop Pandoc
_entirely_ and migrate to a Rust CLI utility that leveraged pulldown-cmark under
the hood for Markdown processing. This version of NDG relied solely on various
hacks (such as Regex) and was not very robust. In NDG 2.0.0, we have migrated to
Comrak and opted in for a more _robust_, AST-based parser powered by
Comrak--which in itself is a very fast Markdown parser.

It is also worth nothing that the official documentation generator for Nixpkgs,
`nixos-render-docs`, uses an in-house tokenizer that is not formally available
elsewhere. This is not a flaw of course, but the lack in the ecosystem lead to
the gradual refactoring of NDG's Markdown parser components into a standalone
crate with increased robustness, better documentation, and generous integration
options for usage by other projects into ecosystems that want to create their
own documentation generators. NDG was eventually updated to use the
`ndg-commonmark`, and has been successfully doing so since the v2.2.0 release.

Thus, the gap in the ecosystem is filled. Rejoice!

> [!TIP]
> While ndg-commonmark _is_ designed _specifically_ for NDG, it is not as
> tightly coupled as it could have been. Which is to say if your use case is not
> supported, or if you have feedback/feature requests you are more than welcome
> to submit those, and we will consider implementing those as time and technical
> constraints allow. ndg-commonmark has been refactored into a library and
> [published on crates.io] for this reason. You may integrate ndg-commonmark
> freely in your utilities as long as you follow the license. It was designed to
> remain compatible no matter now opinionated and unique your tooling is when
> ndg does not meet your requirements or does not just _click_ for your needs.

## Contributing

Most of our projects under the feel-co umbrella, and _especially NDG_, were
designed with our own use cases in mind. While we will not reject any
contributions outright (and in fact, we welcome them nevertheless!), it is often
a good idea to discuss your ideas with us in a friendly issue beforehand. There
may be overlapping plans and scheduling issues that we would rather be prepared
for. As such, please discuss with us before forking or submitting a PR! We may
come to an agreement that benefits us all. Worst case you will be gently nudged
away, or guided into a better compromise that fulfills the needs of everyone.

If making a pull request, please remember to follow our formatting guidelines
and to run Clippy lints. Good API docs are not a hard-requirement, but you
should at least describe the code and explain your motive for any addition or
change.

### Guidelines

NDG has no CLA, and has no formal contributing guidelines. This is because we do
not expect anything from your contribution besides your due diligence. Namely,
we would like you to follow our formatting guidelines (enforced through
`.rustfmt.toml` and `.taplo.toml`) and run Clippy lints.

Comprehensive API docs are not a hard-requirement, but new functions _must_ have
documentation coverage in the sense that they should _at least_ have a comment
explaining what the function does and in the case of ndg-commonmark, some amount
of Rustdoc is good to have. This is not only good for reviewers, but also for
IDE setups where the editor displays function signatures and description in
hover docs. It also tells us that you are fully engaged with the code you're
submitting.

Once your changes are done and you're submitting your PR, please describe your
changes properly and let us know what compelled to make this change. If you can
describe the previous state and the state after your changes, it can help
illustrate your motive better. An empty PR body with no explanation will likely
result in us putting your issue on a backlog until we can engage with it with
comfort.

### Hacking

[Direnv]: https://direnv.net

While working on either Rust crate, you would like some development utilities in
your shell. Even if you do have those in your PATH, it is good to match the
tools we are using with the exact versions we've used. Namely you might need
Cargo, Clippy, Rustfmt and the such consistent with what is used in development.

NDG provides a Nix development shell, which you can use alongside [Direnv] or
manually.

```sh
# Load dependencies
$ direnv allow

# Then you may build the project
$ cargo build --workspace
```

Before submitting a PR, it's a good idea to run Clippy with the following
checks:

- `-W clippy::pedantic`
- `-W clippy::complexity`
- `-W clippy::suspicious`
- `-W clippy::correctness`

The above lints, with some additional rules to allow certain rules, are provided
in `Cargo.toml` in the lints section, so running `cargo clippy` should be enough
unless you have a weird linter setup.

> [!WARNING]
> Do note that usage of `unwrap()` is forbidden in `ndg-commonmark`, and
> `expect()` will cause warnings. Since `ndg-commonmark` is a public library,
> you are expected to use more intuitive error patterns. It is fine to panic in
> `ndg`, but you must ensure that you provide appropriate information with the
> panic. `expect()` is somewhat decent at providing information, and should be
> preferred over `unwrap()` or `panic!`.

#### Usage without flakes

You may interact with our Nix tooling even if you don't want to use Nix flakes.
There exists a compatibility interface for flakes in
[nix/default.nix](./nix/default.nix). Specifically, you can use the following
shell commands:

| With flakes       | Without flakes                  |
| ----------------- | ------------------------------- |
| `nix flake check` | `nix-build nix -A checks`       |
| `nix develop`     | `nix-shell nix -A shell`        |
| `nix build .#ndg` | `nix-build nix -A packages.ndg` |
| `nix fmt`         | `nix run -f nix formatter`      |

## License

<!--markdownlint-disable MD059 -->

[here]: https://www.mozilla.org/en-US/MPL/2.0/

Both NDG and ndg-commonmark are made available under Mozilla Public License
(MPL) version 2.0. See [LICENSE](LICENSE) for more details on the exact
conditions. An online copy is provided [here].

<!--markdownlint-enable MD059 -->
