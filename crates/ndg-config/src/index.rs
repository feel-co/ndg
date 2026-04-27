/// Index page configuration.
///
/// Controls how the site homepage (`index.html`) is generated, including
/// whether to use `README.md` as the homepage and whether to generate a
/// fallback index when no index file is provided.
#[derive(
  Debug, Clone, serde::Serialize, serde::Deserialize, ndg_macros::Configurable,
)]
#[serde(default)]
pub struct IndexConfig {
  /// Whether to use `README.md` as the homepage when `index.md` is not
  /// present.
  ///
  /// When enabled, if the input directory contains a `README.md` but no
  /// `index.md`, the README will be rendered as `index.html` (the site
  /// homepage) instead of `README.html`.
  pub use_readme: bool,

  /// Whether to generate a fallback `index.html` when no index file is
  /// provided.
  ///
  /// When enabled (the default), NDG will create a fallback index page listing
  /// all available documentation pages if no `index.md` or `README.md` (when
  /// `use_readme` is enabled) is present.
  pub generate_fallback: bool,
}

impl Default for IndexConfig {
  fn default() -> Self {
    Self {
      use_readme:        false,
      generate_fallback: true,
    }
  }
}
