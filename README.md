# feel-co/ndg

This repository houses our documentation tooling. The most critical component
stored here is [ndg](./ndg), a super-fast documentation tool to generate a full
fledged website for your Nix-adjacent projects _on the fly_.

## Repository Structure

```plaintext
.
├── ndg
└── ndg-commonmark
```

The ndg directory contains the source code for the ndg utility, `ndg-commonmark`
is a public Rust crate that reimplements Nixpkgs flavors and our own additions
on top of Github Flavored Markdown (GFM). While ndg-commonmark is not exactly
designed for public usage, it was designed with possible users in mind and the
API is generously documented.

### ndg

ndg; or, "Not a Docs Generator" is a in-house documentation utility for Nix and
Nix adjacent projects. It aims to be more batteries included compared to nixpkgs
tooling, and offers a higher degree of customization as well as better error
handling, more graceful error recovery and room for extension.

While the main focus of this tool is to generate documentation for NixOS
modules, you can _easily_ use it as a MdBook replacement in your projects. ndg
offers a flexible parser and configuration options to allow designing documents
for your projects.

### ndg-commonmark

[published on crates.io]: https://crates.io/crates/ndg-commonmark

While designing ndg, a lot of Markdown processing functionality had to be baked
in because there is no library that helps you parse nixpkgs-flavored Markdown in
your tools. The official documentation generator, `nixos-render-docs`, uses an
in-house tokenizer that is not formally available elsewhere. This lack in the
ecosystem lead to the eventual filtering out of ndg's Markdown parser while
increasing its robustness and compatibility with 3rd party tooling.

You may integrate ndg-commonmark in your utilities to implement your own,
opinionated and unique documentation generators if ndg is not up your alley. We
don't judge.

Please note, while ndg-commonmark _is_ designed for ndg, it is not as tightly
coupled as it could have been. If your usecase is not supported, please let us
know in an issue and we'll see what we can do. ndg-commonmark has been
refactored as a library and [published on crates.io] for this reason.

## Contributing

Please understand that our projects are designed with our use-cases in mind.
While we do not reject any contributions outright, it is a good idea to discuss
your ideas with us in a friendly issue beforehand. Worst case you will be gently
nudged away.

If making a pull request, please remember to follow our formatting guidelines
and to run Clippy lints. Good API docs are not a hard-requirement, but you
should at least describe the code and explain your motive for any addition or
change.

## License

Both [ndg](./ndg) and [ndg-commonmark](./ndg-commonmark) are licensed under
Mozilla Public License v2.0. See [LICENSE](./LICENSE) for more details.
