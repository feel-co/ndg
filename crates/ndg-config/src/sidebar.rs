use ndg_macros::Configurable;
use regex::Regex;
use serde::{
  Deserialize,
  Deserializer,
  Serialize,
  de::{self, MapAccess, Visitor},
};

/// Deserializer for match types with `exact` and `regex` fields.
fn deserialize_match_field<'de, D, T>(
  deserializer: D,
  field_name: &'static str,
) -> Result<T, D::Error>
where
  D: Deserializer<'de>,
  T: MatchField,
{
  struct GenericMatchVisitor<T> {
    field_name: &'static str,
    _phantom:   std::marker::PhantomData<T>,
  }

  impl<'de, T: MatchField> Visitor<'de> for GenericMatchVisitor<T> {
    type Value = T;

    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      write!(
        formatter,
        "a string or a map with 'exact' and/or 'regex' fields for {}",
        self.field_name
      )
    }

    fn visit_str<E>(self, value: &str) -> Result<T, E>
    where
      E: de::Error,
    {
      Ok(T::from_exact(value.to_string()))
    }

    fn visit_map<M>(self, mut map: M) -> Result<T, M::Error>
    where
      M: MapAccess<'de>,
    {
      let mut exact = None;
      let mut regex = None;

      while let Some(key) = map.next_key::<String>()? {
        match key.as_str() {
          "exact" => {
            if exact.is_some() {
              return Err(de::Error::duplicate_field("exact"));
            }
            exact = Some(map.next_value()?);
          },
          "regex" => {
            if regex.is_some() {
              return Err(de::Error::duplicate_field("regex"));
            }
            regex = Some(map.next_value()?);
          },
          _ => {
            return Err(de::Error::unknown_field(&key, &["exact", "regex"]));
          },
        }
      }

      Ok(T::from_parts(exact, regex))
    }
  }

  deserializer.deserialize_any(GenericMatchVisitor {
    field_name,
    _phantom: std::marker::PhantomData,
  })
}

/// Trait for match types that can be deserialized generically.
trait MatchField: Sized {
  /// Create from exact match string (for shorthand deserialization).
  fn from_exact(exact: String) -> Self;

  /// Create from optional exact and regex parts.
  fn from_parts(exact: Option<String>, regex: Option<String>) -> Self;
}

/// Configuration for sidebar behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Configurable)]
pub struct SidebarConfig {
  /// Whether to number sidebar items.
  #[serde(default)]
  pub numbered: bool,

  /// Whether to include special files in numbering.
  /// Only has effect when `numbered` is `true`.
  #[serde(default)]
  pub number_special_files: bool,

  /// Ordering algorithm for sidebar items.
  #[serde(default)]
  pub ordering: SidebarOrdering,

  /// Pattern-based matching rules for sidebar items.
  #[serde(default)]
  pub matches: Vec<SidebarMatch>,

  /// Options sidebar configuration.
  #[serde(default)]
  pub options: Option<OptionsConfig>,
}

impl SidebarConfig {
  /// Validate and compile all regex patterns in the sidebar configuration.
  ///
  /// This pre-compiles all regex patterns to ensure they're valid,
  /// failing fast at config load time rather than during rendering.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn validate(&mut self) -> Result<(), String> {
    for (idx, m) in self.matches.iter_mut().enumerate() {
      m.compile_regexes()
        .map_err(|e| format!("Sidebar match #{}: {}", idx + 1, e))?;
    }

    // Validate and compile options config if present
    if let Some(ref mut options_config) = self.options {
      options_config.validate()?;
    }

    Ok(())
  }

  /// Find the first matching rule for the given path and title.
  #[must_use]
  pub fn find_match(&self, path: &str, title: &str) -> Option<&SidebarMatch> {
    self.matches.iter().find(|m| m.matches(path, title))
  }
}

/// Ordering algorithm to use for sidebar items.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SidebarOrdering {
  /// Sort alphabetically by title.
  #[default]
  Alphabetical,

  /// Preserve filesystem ordering.
  Filesystem,

  /// Use custom ordering via position field.
  Custom,
}

impl std::str::FromStr for SidebarOrdering {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "alphabetical" => Ok(Self::Alphabetical),
      "filesystem" => Ok(Self::Filesystem),
      "custom" => Ok(Self::Custom),
      _ => Err(format!("Unknown sidebar ordering: {s}")),
    }
  }
}

/// Path matching criteria
#[derive(Debug, Clone, Serialize, Default)]
pub struct PathMatch {
  /// Exact path match.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exact: Option<String>,

  /// Regex pattern for path matching.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub regex: Option<String>,

  /// Compiled regex cache (populated after validation).
  #[serde(skip)]
  pub compiled_regex: Option<Regex>,
}

impl PartialEq for PathMatch {
  fn eq(&self, other: &Self) -> bool {
    self.exact == other.exact && self.regex == other.regex
  }
}

impl Eq for PathMatch {}

impl MatchField for PathMatch {
  fn from_exact(exact: String) -> Self {
    Self {
      exact:          Some(exact),
      regex:          None,
      compiled_regex: None,
    }
  }

  fn from_parts(exact: Option<String>, regex: Option<String>) -> Self {
    Self {
      exact,
      regex,
      compiled_regex: None,
    }
  }
}

impl PathMatch {
  /// Create a new [`PathMatch`] with exact matching.
  #[cfg(test)]
  #[must_use]
  pub const fn exact(path: String) -> Self {
    Self {
      exact:          Some(path),
      regex:          None,
      compiled_regex: None,
    }
  }

  /// Create a new [`PathMatch`] with regex matching.
  #[cfg(test)]
  #[must_use]
  pub const fn regex(pattern: String) -> Self {
    Self {
      exact:          None,
      regex:          Some(pattern),
      compiled_regex: None,
    }
  }
}

impl<'de> Deserialize<'de> for PathMatch {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserialize_match_field(deserializer, "path")
  }
}

/// Title matching criteria (exact or regex).
#[derive(Debug, Clone, Serialize, Default)]
pub struct TitleMatch {
  /// Exact title match.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exact: Option<String>,

  /// Regex pattern for title matching.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub regex: Option<String>,

  /// Compiled regex cache (populated after validation).
  #[serde(skip)]
  pub compiled_regex: Option<Regex>,
}

impl PartialEq for TitleMatch {
  fn eq(&self, other: &Self) -> bool {
    self.exact == other.exact && self.regex == other.regex
  }
}

impl Eq for TitleMatch {}

impl MatchField for TitleMatch {
  fn from_exact(exact: String) -> Self {
    Self {
      exact:          Some(exact),
      regex:          None,
      compiled_regex: None,
    }
  }

  fn from_parts(exact: Option<String>, regex: Option<String>) -> Self {
    Self {
      exact,
      regex,
      compiled_regex: None,
    }
  }
}

impl TitleMatch {
  /// Create a new `TitleMatch` with exact matching.
  #[cfg(test)]
  #[must_use]
  pub const fn exact(title: String) -> Self {
    Self {
      exact:          Some(title),
      regex:          None,
      compiled_regex: None,
    }
  }

  /// Create a new [`TitleMatch`] with regex matching.
  #[cfg(test)]
  #[must_use]
  pub const fn regex(pattern: String) -> Self {
    Self {
      exact:          None,
      regex:          Some(pattern),
      compiled_regex: None,
    }
  }
}

impl<'de> Deserialize<'de> for TitleMatch {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserialize_match_field(deserializer, "title")
  }
}

/// Pattern-based matching rule for sidebar items.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SidebarMatch {
  /// Path matching criteria.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<PathMatch>,

  /// Title matching criteria.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<TitleMatch>,

  /// Override title with this value.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub new_title: Option<String>,

  /// Custom position in sidebar.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub position: Option<usize>,
}

impl SidebarMatch {
  /// Compile all regex patterns in this match rule.
  ///
  /// This must be called after deserialization to populate the compiled regex
  /// cache. Regexes are validated and compiled once, then reused for all
  /// subsequent match operations.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn compile_regexes(&mut self) -> Result<(), String> {
    if let Some(ref mut path_match) = self.path
      && let Some(ref pattern) = path_match.regex
    {
      path_match.compiled_regex = Some(
        Regex::new(pattern)
          .map_err(|e| format!("Invalid path regex '{pattern}': {e}"))?,
      );
    }

    if let Some(ref mut title_match) = self.title
      && let Some(ref pattern) = title_match.regex
    {
      title_match.compiled_regex = Some(
        Regex::new(pattern)
          .map_err(|e| format!("Invalid title regex '{pattern}': {e}"))?,
      );
    }

    Ok(())
  }

  /// Check if this rule matches the given path and title.
  ///
  /// All specified conditions must match.
  ///
  /// # Panics
  ///
  /// Panics if `compile_regexes()` was not called after deserialization and
  /// regex patterns are present. This is a programming error - configs should
  /// always be validated/compiled via `SidebarConfig::validate()`.
  #[must_use]
  pub fn matches(&self, path_str: &str, title_str: &str) -> bool {
    // Check path matching
    if let Some(ref path_match) = self.path {
      // Check exact path match
      if let Some(ref exact_path) = path_match.exact
        && path_str != exact_path
      {
        return false;
      }

      // Check regex path match
      if let Some(ref _pattern) = path_match.regex {
        #[allow(clippy::expect_used)]
        let re = path_match
          .compiled_regex
          .as_ref()
          .expect("internal error: invalid regex configuration");
        if !re.is_match(path_str) {
          return false;
        }
      }
    }

    // Check title matching
    if let Some(ref title_match) = self.title {
      // Check exact title match
      if let Some(ref exact_title) = title_match.exact
        && title_str != exact_title
      {
        return false;
      }

      // Check regex title match
      if let Some(ref _pattern) = title_match.regex {
        #[allow(clippy::expect_used)]
        let re = title_match
          .compiled_regex
          .as_ref()
          .expect("internal error: invalid regex configuration");
        if !re.is_match(title_str) {
          return false;
        }
      }
    }

    true
  }

  /// Get the position for this match.
  #[must_use]
  pub const fn get_position(&self) -> Option<usize> {
    self.position
  }

  /// Get the custom title for this match.
  #[must_use]
  pub fn get_title(&self) -> Option<&str> {
    self.new_title.as_deref()
  }
}

/// Configuration for options sidebar behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Configurable)]
#[serde(default)]
pub struct OptionsConfig {
  /// Depth of parent categories in options TOC.
  #[config(key = "depth")]
  pub depth: usize,

  /// Ordering algorithm for options.
  #[config(key = "ordering")]
  pub ordering: SidebarOrdering,

  /// Pattern-based matching rules for options.
  pub matches: Vec<OptionsMatch>,
}

impl Default for OptionsConfig {
  fn default() -> Self {
    Self {
      depth:    2,
      ordering: SidebarOrdering::default(),
      matches:  Vec::new(),
    }
  }
}

impl OptionsConfig {
  /// Validate and compile all regex patterns in the options configuration.
  ///
  /// Pre-compiles all regex patterns to ensure they're valid,
  /// failing fast at config load time rather than during rendering.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn validate(&mut self) -> Result<(), String> {
    for (idx, m) in self.matches.iter_mut().enumerate() {
      m.compile_regexes()
        .map_err(|e| format!("Options match #{}: {}", idx + 1, e))?;
    }
    Ok(())
  }

  /// Find the first matching rule for the given option name.
  #[must_use]
  pub fn find_match(&self, option_name: &str) -> Option<&OptionsMatch> {
    self.matches.iter().find(|m| m.matches(option_name))
  }
}

/// Option name matching criteria (exact or regex).
#[derive(Debug, Clone, Serialize, Default)]
pub struct OptionNameMatch {
  /// Exact option name match.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exact: Option<String>,

  /// Regex pattern for option name matching.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub regex: Option<String>,

  /// Compiled regex cache (populated after validation).
  #[serde(skip)]
  pub compiled_regex: Option<Regex>,
}

impl PartialEq for OptionNameMatch {
  fn eq(&self, other: &Self) -> bool {
    self.exact == other.exact && self.regex == other.regex
  }
}

impl Eq for OptionNameMatch {}

impl MatchField for OptionNameMatch {
  fn from_exact(exact: String) -> Self {
    Self {
      exact:          Some(exact),
      regex:          None,
      compiled_regex: None,
    }
  }

  fn from_parts(exact: Option<String>, regex: Option<String>) -> Self {
    Self {
      exact,
      regex,
      compiled_regex: None,
    }
  }
}

impl OptionNameMatch {
  /// Create a new [`OptionNameMatch`] with exact matching.
  #[cfg(test)]
  #[must_use]
  pub const fn exact(name: String) -> Self {
    Self {
      exact:          Some(name),
      regex:          None,
      compiled_regex: None,
    }
  }

  /// Create a new [`OptionNameMatch`] with regex matching.
  #[cfg(test)]
  #[must_use]
  pub const fn regex(pattern: String) -> Self {
    Self {
      exact:          None,
      regex:          Some(pattern),
      compiled_regex: None,
    }
  }
}

impl<'de> Deserialize<'de> for OptionNameMatch {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserialize_match_field(deserializer, "name")
  }
}

/// Matching rule for options.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OptionsMatch {
  /// Option name matching criteria.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<OptionNameMatch>,

  /// Override display name with this value.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub new_name: Option<String>,

  /// Custom grouping depth for this option.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub depth: Option<usize>,

  /// Custom position in sidebar.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub position: Option<usize>,

  /// Hide this option from the TOC.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hidden: Option<bool>,
}

impl OptionsMatch {
  /// Compile all regex patterns in this match rule.
  ///
  /// This must be called after deserialization to populate the compiled regex
  /// cache. Regexes are validated and compiled once, then reused for all
  /// subsequent match operations.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn compile_regexes(&mut self) -> Result<(), String> {
    if let Some(ref mut name_match) = self.name
      && let Some(ref pattern) = name_match.regex
    {
      name_match.compiled_regex = Some(
        Regex::new(pattern)
          .map_err(|e| format!("Invalid name regex '{pattern}': {e}"))?,
      );
    }

    Ok(())
  }

  /// Check if this rule matches the given option name.
  ///
  /// All specified conditions must match (AND logic).
  ///
  /// # Panics
  ///
  /// Panics if `compile_regexes()` was not called after deserialization and
  /// regex patterns are present. This is a programming error - configs should
  /// always be validated/compiled via `OptionsConfig::validate()`.
  #[must_use]
  pub fn matches(&self, option_name: &str) -> bool {
    // Check name matching
    if let Some(ref name_match) = self.name {
      // Check exact name match
      if let Some(ref exact_name) = name_match.exact
        && option_name != exact_name
      {
        return false;
      }

      // Check regex name match
      if let Some(ref _pattern) = name_match.regex {
        #[allow(clippy::expect_used)]
        let re = name_match
          .compiled_regex
          .as_ref()
          .expect("internal error: invalid regex configuration");
        if !re.is_match(option_name) {
          return false;
        }
      }
    }

    true
  }

  /// Get the custom display name for this option, if set.
  #[must_use]
  pub fn get_name(&self) -> Option<&str> {
    self.new_name.as_deref()
  }

  /// Get the custom depth for this option, if set.
  #[must_use]
  pub const fn get_depth(&self) -> Option<usize> {
    self.depth
  }

  /// Get the custom position for this option, if set.
  #[must_use]
  pub const fn get_position(&self) -> Option<usize> {
    self.position
  }

  /// Check if this option should be hidden from the TOC.
  #[must_use]
  pub const fn is_hidden(&self) -> bool {
    match self.hidden {
      Some(hidden) => hidden,
      None => false,
    }
  }
}

#[cfg(test)]
mod tests {
  #![allow(clippy::expect_used, reason = "Fine in tests")]

  use super::*;

  #[test]
  fn test_sidebar_ordering_deserialization() {
    #[derive(Deserialize)]
    struct Wrapper {
      ordering: SidebarOrdering,
    }

    let toml = r#"ordering = "alphabetical""#;
    let wrapper: Wrapper =
      toml::from_str(toml).expect("Failed to parse alphabetical ordering TOML");
    assert!(matches!(wrapper.ordering, SidebarOrdering::Alphabetical));

    let toml = r#"ordering = "custom""#;
    let wrapper: Wrapper =
      toml::from_str(toml).expect("Failed to parse custom ordering TOML");
    assert!(matches!(wrapper.ordering, SidebarOrdering::Custom));

    let toml = r#"ordering = "filesystem""#;
    let wrapper: Wrapper =
      toml::from_str(toml).expect("Failed to parse filesystem ordering TOML");
    assert!(matches!(wrapper.ordering, SidebarOrdering::Filesystem));
  }

  #[test]
  fn test_sidebar_config_default() {
    let config = SidebarConfig::default();
    assert!(!config.numbered);
    assert!(!config.number_special_files);
    assert!(matches!(config.ordering, SidebarOrdering::Alphabetical));
    assert!(config.matches.is_empty());
    assert!(config.options.is_none());
  }

  #[test]
  fn test_sidebar_match_exact_path() {
    let m = SidebarMatch {
      path:      Some(PathMatch::exact("getting-started.md".to_string())),
      title:     None,
      new_title: None,
      position:  Some(1),
    };

    assert!(m.matches("getting-started.md", "Any Title"));
    assert!(!m.matches("other.md", "Any Title"));
  }

  #[test]
  fn test_sidebar_match_regex_path() {
    let mut m = SidebarMatch {
      path:      Some(PathMatch::regex(r"^api/.*\.md$".to_string())),
      title:     None,
      new_title: None,
      position:  Some(50),
    };

    m.compile_regexes().expect("regex should compile");

    assert!(m.matches("api/foo.md", "Any Title"));
    assert!(m.matches("api/bar/baz.md", "Any Title"));
    assert!(!m.matches("other.md", "Any Title"));
  }

  #[test]
  fn test_sidebar_match_exact_title() {
    let m = SidebarMatch {
      path:      None,
      title:     Some(TitleMatch::exact("Release Notes".to_string())),
      new_title: Some("What's New".to_string()),
      position:  Some(999),
    };

    assert!(m.matches("any/path.md", "Release Notes"));
    assert!(!m.matches("any/path.md", "Other Title"));
  }

  #[test]
  fn test_sidebar_match_regex_title() {
    let mut m = SidebarMatch {
      path:      None,
      title:     Some(TitleMatch::regex(r"^Release.*".to_string())),
      new_title: Some("What's New".to_string()),
      position:  Some(999),
    };

    m.compile_regexes().expect("regex should compile");

    assert!(m.matches("any/path.md", "Release Notes"));
    assert!(m.matches("any/path.md", "Release 1.0"));
    assert!(!m.matches("any/path.md", "Other Title"));
  }

  #[test]
  fn test_sidebar_match_combined_conditions() {
    let mut m = SidebarMatch {
      path:      Some(PathMatch::regex(r"^api/.*\.md$".to_string())),
      title:     Some(TitleMatch::regex(r"^API.*".to_string())),
      new_title: None,
      position:  Some(50),
    };

    m.compile_regexes().expect("regexes should compile");

    // Both conditions must match
    assert!(m.matches("api/foo.md", "API Reference"));
    assert!(!m.matches("api/foo.md", "Other Title"));
    assert!(!m.matches("other.md", "API Reference"));
  }

  #[test]
  fn test_sidebar_match_get_position() {
    let m = SidebarMatch {
      path:      Some(PathMatch::exact("test.md".to_string())),
      title:     None,
      new_title: None,
      position:  Some(42),
    };

    assert_eq!(m.get_position(), Some(42));
  }

  #[test]
  fn test_sidebar_match_get_title() {
    let m = SidebarMatch {
      path:      Some(PathMatch::exact("test.md".to_string())),
      title:     None,
      new_title: Some("Custom Title".to_string()),
      position:  None,
    };

    assert_eq!(m.get_title(), Some("Custom Title"));
  }

  #[test]
  fn test_sidebar_config_find_match() {
    let mut config = SidebarConfig {
      numbered:             true,
      number_special_files: false,
      ordering:             SidebarOrdering::Custom,
      options:              None,
      matches:              vec![
        SidebarMatch {
          path:      Some(PathMatch::exact("getting-started.md".to_string())),
          title:     None,
          new_title: None,
          position:  Some(1),
        },
        SidebarMatch {
          path:      Some(PathMatch::regex(r"^api/.*\.md$".to_string())),
          title:     None,
          new_title: None,
          position:  Some(50),
        },
      ],
    };

    // Validate to compile regexes
    config.validate().expect("config should be valid");

    // First rule matches
    assert!(config.find_match("getting-started.md", "Title").is_some());

    // Second rule matches
    assert!(config.find_match("api/foo.md", "Title").is_some());

    // No match
    assert!(config.find_match("other.md", "Title").is_none());
  }

  #[test]
  fn test_sidebar_config_first_rule_wins() {
    let config = SidebarConfig {
      numbered:             false,
      number_special_files: false,
      ordering:             SidebarOrdering::Alphabetical,
      options:              None,
      matches:              vec![
        SidebarMatch {
          path:      Some(PathMatch::exact("test.md".to_string())),
          title:     None,
          new_title: Some("First".to_string()),
          position:  Some(1),
        },
        SidebarMatch {
          path:      Some(PathMatch::exact("test.md".to_string())),
          title:     None,
          new_title: Some("Second".to_string()),
          position:  Some(2),
        },
      ],
    };

    let m = config
      .find_match("test.md", "Title")
      .expect("Should match test.md");
    assert_eq!(m.new_title.as_deref(), Some("First"));
    assert_eq!(m.position, Some(1));
  }

  #[test]
  fn test_sidebar_config_get_position() {
    let config = SidebarConfig {
      numbered:             false,
      number_special_files: false,
      ordering:             SidebarOrdering::Alphabetical,
      options:              None,
      matches:              vec![SidebarMatch {
        path:      Some(PathMatch::exact("test.md".to_string())),
        title:     None,
        new_title: None,
        position:  Some(42),
      }],
    };

    assert_eq!(
      config
        .find_match("test.md", "Title")
        .and_then(super::SidebarMatch::get_position),
      Some(42)
    );
    assert_eq!(config.find_match("other.md", "Title"), None);
  }

  #[test]
  fn test_sidebar_config_get_title() {
    let config = SidebarConfig {
      numbered:             false,
      number_special_files: false,
      ordering:             SidebarOrdering::Alphabetical,
      options:              None,
      matches:              vec![SidebarMatch {
        path:      Some(PathMatch::exact("test.md".to_string())),
        title:     None,
        new_title: Some("Custom".to_string()),
        position:  None,
      }],
    };

    assert_eq!(
      config
        .find_match("test.md", "Title")
        .and_then(|m| m.get_title()),
      Some("Custom")
    );
    assert_eq!(config.find_match("other.md", "Title"), None);
  }

  #[test]
  fn test_sidebar_config_toml_deserialization() {
    let toml = r#"
numbered = true
ordering = "custom"

[[matches]]
path.exact = "getting-started.md"
position = 1

[[matches]]
path.regex = "^api/.*\\.md$"
position = 50
"#;

    let config: SidebarConfig =
      toml::from_str(toml).expect("Failed to parse sidebar TOML config");
    assert!(config.numbered);
    assert!(matches!(config.ordering, SidebarOrdering::Custom));
    assert_eq!(config.matches.len(), 2);

    // First match
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.exact.as_deref()),
      Some("getting-started.md")
    );
    assert_eq!(config.matches[0].position, Some(1));

    // Second match
    assert_eq!(
      config.matches[1]
        .path
        .as_ref()
        .and_then(|p| p.regex.as_deref()),
      Some(r"^api/.*\.md$")
    );
    assert_eq!(config.matches[1].position, Some(50));
  }

  #[test]
  fn test_sidebar_config_json_deserialization() {
    let json = r#"{
  "numbered": true,
  "ordering": "alphabetical",
  "matches": [
    {
      "path": {
        "exact": "getting-started.md"
      },
      "position": 1
    },
    {
      "path": {
        "regex": "^api/.*\\.md$"
      },
      "position": 50
    }
  ]
}"#;

    let config: SidebarConfig =
      serde_json::from_str(json).expect("Failed to parse sidebar JSON config");
    assert!(config.numbered);
    assert!(matches!(config.ordering, SidebarOrdering::Alphabetical));
    assert_eq!(config.matches.len(), 2);

    // First match
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.exact.as_deref()),
      Some("getting-started.md")
    );
    assert_eq!(config.matches[0].position, Some(1));

    // Second match
    assert_eq!(
      config.matches[1]
        .path
        .as_ref()
        .and_then(|p| p.regex.as_deref()),
      Some(r"^api/.*\.md$")
    );
    assert_eq!(config.matches[1].position, Some(50));
  }

  #[test]
  fn test_path_match_shorthand_string() {
    let toml = r#"
[[matches]]
path = "getting-started.md"
position = 1
"#;

    let config: SidebarConfig = toml::from_str(toml)
      .expect("Failed to parse sidebar TOML with path shorthand");
    assert_eq!(config.matches.len(), 1);

    // Path shorthand should be converted to PathMatch with exact field
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.exact.as_deref()),
      Some("getting-started.md")
    );
    assert_eq!(config.matches[0].position, Some(1));
  }

  #[test]
  fn test_json_shorthand() {
    let json = r#"{
  "numbered": true,
  "matches": [
    {
      "path": "getting-started.md",
      "position": 1
    }
  ]
}"#;

    let config: SidebarConfig = serde_json::from_str(json)
      .expect("Failed to parse sidebar JSON with shorthand");
    assert_eq!(config.matches.len(), 1);

    // JSON shorthand should also work
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.exact.as_deref()),
      Some("getting-started.md")
    );
  }

  // Options configuration tests

  #[test]
  fn test_options_config_default() {
    let config = OptionsConfig::default();
    assert_eq!(config.depth, 2);
    assert!(matches!(config.ordering, SidebarOrdering::Alphabetical));
    assert!(config.matches.is_empty());
  }

  #[test]
  fn test_options_match_exact_name() {
    let m = OptionsMatch {
      name:     Some(OptionNameMatch::exact(
        "programs.neovim.enable".to_string(),
      )),
      new_name: Some("Neovim".to_string()),
      depth:    None,
      position: Some(1),
      hidden:   None,
    };

    assert!(m.matches("programs.neovim.enable"));
    assert!(!m.matches("programs.vim.enable"));
  }

  #[test]
  fn test_options_match_regex_name() {
    let mut m = OptionsMatch {
      name:     Some(OptionNameMatch::regex(r"^programs\..*".to_string())),
      new_name: Some("Programs".to_string()),
      depth:    Some(1),
      position: Some(1),
      hidden:   None,
    };

    m.compile_regexes().expect("regex should compile");

    assert!(m.matches("programs.neovim.enable"));
    assert!(m.matches("programs.vim.enable"));
    assert!(!m.matches("services.nginx.enable"));
  }

  #[test]
  fn test_options_match_hidden() {
    let m = OptionsMatch {
      name:     Some(OptionNameMatch::exact("internal.option".to_string())),
      new_name: None,
      depth:    None,
      position: None,
      hidden:   Some(true),
    };

    assert!(m.matches("internal.option"));
    assert!(m.is_hidden());
  }

  #[test]
  fn test_options_match_getters() {
    let m = OptionsMatch {
      name:     Some(OptionNameMatch::exact("test.option".to_string())),
      new_name: Some("Test Option".to_string()),
      depth:    Some(3),
      position: Some(42),
      hidden:   Some(false),
    };

    assert_eq!(m.get_name(), Some("Test Option"));
    assert_eq!(m.get_depth(), Some(3));
    assert_eq!(m.get_position(), Some(42));
    assert!(!m.is_hidden());
  }

  #[test]
  fn test_options_config_find_match() {
    let mut config = OptionsConfig {
      depth:    2,
      ordering: SidebarOrdering::Custom,
      matches:  vec![
        OptionsMatch {
          name:     Some(OptionNameMatch::exact(
            "programs.neovim.enable".to_string(),
          )),
          new_name: Some("Neovim".to_string()),
          depth:    None,
          position: Some(1),
          hidden:   None,
        },
        OptionsMatch {
          name:     Some(OptionNameMatch::regex(r"^services\..*".to_string())),
          new_name: Some("Services".to_string()),
          depth:    Some(1),
          position: Some(50),
          hidden:   None,
        },
      ],
    };

    // Validate to compile regexes
    config.validate().expect("config should be valid");

    // First rule matches
    assert!(config.find_match("programs.neovim.enable").is_some());

    // Second rule matches
    assert!(config.find_match("services.nginx.enable").is_some());

    // No match
    assert!(config.find_match("other.option").is_none());
  }

  #[test]
  fn test_options_config_first_rule_wins() {
    let config = OptionsConfig {
      depth:    2,
      ordering: SidebarOrdering::Alphabetical,
      matches:  vec![
        OptionsMatch {
          name:     Some(OptionNameMatch::exact("test.option".to_string())),
          new_name: Some("First".to_string()),
          depth:    None,
          position: Some(1),
          hidden:   None,
        },
        OptionsMatch {
          name:     Some(OptionNameMatch::exact("test.option".to_string())),
          new_name: Some("Second".to_string()),
          depth:    None,
          position: Some(2),
          hidden:   None,
        },
      ],
    };

    let m = config
      .find_match("test.option")
      .expect("Should match test.option");
    assert_eq!(m.new_name.as_deref(), Some("First"));
    assert_eq!(m.position, Some(1));
  }

  #[test]
  fn test_option_name_match_shorthand_string() {
    let toml = r#"
[[matches]]
name = "programs.neovim.enable"
position = 1
"#;

    let config: OptionsConfig = toml::from_str(&format!("[options]\n{toml}"))
      .expect("Failed to parse options TOML with shorthand name");
    assert_eq!(config.matches.len(), 1);

    // Shorthand string should become exact match
    assert_eq!(
      config.matches[0]
        .name
        .as_ref()
        .and_then(|n| n.exact.as_deref()),
      Some("programs.neovim.enable")
    );
    assert_eq!(
      config.matches[0]
        .name
        .as_ref()
        .and_then(|n| n.regex.as_ref()),
      None
    );
  }

  #[test]
  fn test_options_config_toml_deserialization() {
    let toml = r#"
depth = 3
ordering = "custom"

[[matches]]
name.exact = "programs.neovim.enable"
new_name = "Neovim"
position = 1

[[matches]]
name.regex = "^services\\..*"
depth = 1
position = 50
"#;

    let config: OptionsConfig =
      toml::from_str(toml).expect("Failed to parse options TOML config");
    assert_eq!(config.depth, 3);
    assert!(matches!(config.ordering, SidebarOrdering::Custom));
    assert_eq!(config.matches.len(), 2);

    // First match
    assert_eq!(
      config.matches[0]
        .name
        .as_ref()
        .and_then(|n| n.exact.as_deref()),
      Some("programs.neovim.enable")
    );
    assert_eq!(config.matches[0].new_name.as_deref(), Some("Neovim"));
    assert_eq!(config.matches[0].position, Some(1));

    // Second match
    assert_eq!(
      config.matches[1]
        .name
        .as_ref()
        .and_then(|n| n.regex.as_deref()),
      Some(r"^services\..*")
    );
    assert_eq!(config.matches[1].depth, Some(1));
    assert_eq!(config.matches[1].position, Some(50));
  }

  #[test]
  fn test_options_config_json_deserialization() {
    let json = r#"{
  "depth": 3,
  "ordering": "alphabetical",
  "matches": [
    {
      "name": {
        "exact": "programs.neovim.enable"
      },
      "new_name": "Neovim",
      "position": 1
    },
    {
      "name": {
        "regex": "^services\\..*"
      },
      "depth": 1,
      "hidden": true
    }
  ]
}"#;

    let config: OptionsConfig =
      serde_json::from_str(json).expect("Failed to parse options JSON config");
    assert_eq!(config.depth, 3);
    assert!(matches!(config.ordering, SidebarOrdering::Alphabetical));
    assert_eq!(config.matches.len(), 2);

    // First match
    assert_eq!(
      config.matches[0]
        .name
        .as_ref()
        .and_then(|n| n.exact.as_deref()),
      Some("programs.neovim.enable")
    );
    assert_eq!(config.matches[0].new_name.as_deref(), Some("Neovim"));

    // Second match
    assert_eq!(
      config.matches[1]
        .name
        .as_ref()
        .and_then(|n| n.regex.as_deref()),
      Some(r"^services\..*")
    );
    assert_eq!(config.matches[1].hidden, Some(true));
  }

  #[test]
  fn test_options_config_validation_invalid_regex() {
    let mut config = OptionsConfig {
      depth:    2,
      ordering: SidebarOrdering::Alphabetical,
      matches:  vec![OptionsMatch {
        name:     Some(OptionNameMatch::regex("[invalid regex(".to_string())),
        new_name: None,
        depth:    None,
        position: None,
        hidden:   None,
      }],
    };

    let result = config.validate();
    assert!(result.is_err());
    let error_msg = result.expect_err("Should have validation error");
    assert!(
      error_msg.contains("Options match #1"),
      "Error should mention match number: {error_msg}"
    );
    assert!(
      error_msg.contains("Invalid name regex"),
      "Error should mention invalid regex: {error_msg}"
    );
  }

  // Tests for proc-macro-based config system

  #[test]
  fn test_options_config_apply_override_depth() {
    let mut config = OptionsConfig::default();
    assert_eq!(config.depth, 2);

    config.apply_override("depth", "5").unwrap();
    assert_eq!(config.depth, 5);
  }

  #[test]
  fn test_options_config_apply_override_ordering() {
    let mut config = OptionsConfig::default();
    assert!(matches!(config.ordering, SidebarOrdering::Alphabetical));

    config.apply_override("ordering", "custom").unwrap();
    assert!(matches!(config.ordering, SidebarOrdering::Custom));

    config.apply_override("ordering", "filesystem").unwrap();
    assert!(matches!(config.ordering, SidebarOrdering::Filesystem));
  }

  #[test]
  fn test_options_config_apply_override_invalid_ordering() {
    let mut config = OptionsConfig::default();

    let result = config.apply_override("ordering", "invalid");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Unknown sidebar ordering"));
  }

  #[test]
  fn test_options_config_merge_fields() {
    let mut config = OptionsConfig::default();
    config.depth = 2;
    config.ordering = SidebarOrdering::Alphabetical;

    let other = OptionsConfig {
      depth:    4,
      ordering: SidebarOrdering::Custom,
      matches:  vec![OptionsMatch {
        name: Some(OptionNameMatch::exact("test".to_string())),
        ..Default::default()
      }],
    };

    config.merge_fields(other);

    assert_eq!(config.depth, 4);
    assert!(matches!(config.ordering, SidebarOrdering::Custom));
    // Vec fields should extend
    assert_eq!(config.matches.len(), 1);
  }
}
