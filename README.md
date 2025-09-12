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
