# feel-co/ndg

This repository contains our in-house documentation tooling, designed to
accommodate our various documentation needs, and the relevant dependencies that
are needed to make it happen. The most critical component stored here, which you
might be the most interested in, is [ndg](./ndg); a fast, robust and
customizable documentation utility to generate a complete documentation website
for your Nix-adjacent projects on the fly. [ndg-commonmark](./ndg-commonmark) is
a Rust crate that handles parsing flavored commonmark, which is not very useful
on its own.

If you plan to make use of NDG or the ndg-commonmark crate, please read the
appropriate sections below.

## Repository Structure

```plaintext
.
├── ndg
└── ndg-commonmark
```

[Github Flavored Markdown]: https://github.github.com/gfm/

The `ndg/` directory contains the source code for the **ndg** utility, and
**ndg-commonmark** is a public Rust crate that implements a parser for Nixpkgs
flavored CommonMark with our own additions on top of [Github Flavored Markdown]
(GFM). While ndg-commonmark was not _exactly_ designed for public use, it _was_
designed with possible users and use-cases in mind, and the API was generously
documented.

### ndg

ndg; or, "Not A Docs Generator" is our in-house documentation utility for Nix
and Nix-adjacent projects, replacing commonly used tooling such as MdBook with a
heavier emphasis on Nix and customizability. ndg also aims to be more "batteries
included" compared to nixpkgs tooling (à la `nixos-render-docs`), and offers
various improvements such as a higher degree of customization, graceful error
recovery, and a lot of room for extension as ndg is not tied to Nixpkgs in any
way.

While the main focus of this tool to generate documentation for Nix module
systems, you may _easily_ use it as a MdBook replacement in your projects by
disabling Nix-specific features. ndg offers a flexible Markdown parser, a robust
templating system and various configuration options to allow designing beautiful
documentation for your projects.

### ndg-commonmark

[published on crates.io]: https://crates.io/crates/ndg-commonmark

While working on ndg, a large portion of Markdown parsing had to be done through
various hacks (such as Regex) in house because there is no library that can
parse Nixpkgs-flavored Markdown. The official documentation generator for
Nixpkgs, `nixos-render-docs`, uses an in-house tokenizer that is not formally
available elsewhere. This lack in the ecosystem lead to the gradual refactoring
out of ndg's Markdown parser as a standalone crate while increasing its
robustness, documentation and compatibility offerings for 3rd party tooling. ndg
was eventually updated to use the `ndg-commonmark`.

You may integrate ndg-commonmark freely in your utilities as long as you follow
the license. It was designed to remain compatible no matter now opinionated and
unique your tooling is when ndg does not meet your requirements or does not just
_click_ for your needs.

Please note, while ndg-commonmark _is_ designed for ndg, it is not as tightly
coupled as it could have been. If your usecase is not supported, please let us
know in an issue and we'll see what we can do. ndg-commonmark has been
refactored as a library and [published on crates.io] for this reason.

## Contributing

Most of our projects, and especially ndg, were designed with our own use cases
in mind. While we will not reject any contributions outright (and we welcome
them regardless!), it is often a good idea to discuss your ideas in a friendly
issue beforehand. There may be overlapping plans and scheduling issues that we
would rather be prepared for. As such, please discuss with us before forking or
submitting a PR! We may come to an agreement that benefits us all. Worst case
you will be gently nudged away.

If making a pull request, please remember to follow our formatting guidelines
and to run Clippy lints. Good API docs are not a hard-requirement, but you
should at least describe the code and explain your motive for any addition or
change.

### Guidelines

We do not expect much from you in a contribution besides your due diligence.
Namely, we would like you to follow our formatting guidelines (enforced through
`.rustfmt.toml` and `.taplo.toml`) and run Clippy lints. Good API docs are not a
hard-requirement, but new functions _must_ have documentation coverage in the
sense that they should _at least_ have a comment explaining what the function
does. This is not only good for reviewers, but also for IDE setups where the
editor displays function signatures and description in hover docs.

While submitting a PR, please describe your changes properly and let us know
what compelled to make this change. If you can describe the previous state and
the state after your changes, it can help illustrate your motive better.

### Hacking

While working on either crate, you would like some development utilities in your
shell. Namely you might need Cargo, Clippy, Rustfmt and the such consistent with
what is used in development. NDG provides a Nix development shell, which you can
use alongside [Direnv](https://direnv.net) or manually.

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

#### Usage without flakes

We support usage without flakes via [nix/default.nix](./nix/default.nix).
Specifically, you can use the following shell commands:

| With flakes       | Without flakes                  |
| ----------------- | ------------------------------- |
| `nix flake check` | `nix-build nix -A checks`       |
| `nix develop`     | `nix-shell nix -A shell`        |
| `nix build .#ndg` | `nix-build nix -A packages.ndg` |
| `nix fmt`         | `nix run -f nix formatter`      |

## License

Both [ndg](./ndg) and [ndg-commonmark](./ndg-commonmark) are licensed under
Mozilla Public License v2.0. See [LICENSE](./LICENSE) for more details.
