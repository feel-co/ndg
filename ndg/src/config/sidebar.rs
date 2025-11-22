use regex::Regex;
use serde::{
  Deserialize,
  Deserializer,
  Serialize,
  de::{self, MapAccess, Visitor},
};

/// Configuration for sidebar behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SidebarConfig {
  /// Whether to number sidebar items.
  #[serde(default)]
  pub numbered: bool,

  /// Whether to include special files (index.md, README.md) in numbering.
  /// Only has effect when `numbered` is true.
  #[serde(default)]
  pub number_special_files: bool,

  /// Ordering algorithm for sidebar items.
  #[serde(default)]
  pub ordering: SidebarOrdering,

  /// Pattern-based matching rules for sidebar items.
  #[serde(default)]
  pub matches: Vec<SidebarMatch>,
}

impl SidebarConfig {
  /// Validate all regex patterns in the sidebar configuration.
  ///
  /// This pre-compiles all regex patterns to ensure they're valid,
  /// failing fast at config load time rather than during rendering.
  ///
  /// # Errors
  ///
  /// Returns an error if any regex pattern is invalid.
  pub fn validate(&self) -> Result<(), String> {
    for (idx, m) in self.matches.iter().enumerate() {
      // Validate path regex if present
      if let Some(ref path_match) = m.path {
        if let Some(ref regex_path) = path_match.regex {
          Regex::new(regex_path).map_err(|e| {
            format!(
              "Invalid path regex pattern in sidebar match #{}: '{}' - {}",
              idx + 1,
              regex_path,
              e
            )
          })?;
        }
      }

      // Validate title regex if present
      if let Some(ref title_match) = m.title {
        if let Some(ref regex_title) = title_match.regex {
          Regex::new(regex_title).map_err(|e| {
            format!(
              "Invalid title regex pattern in sidebar match #{}: '{}' - {}",
              idx + 1,
              regex_title,
              e
            )
          })?;
        }
      }
    }
    Ok(())
  }

  /// Find the first matching rule for the given path and title.
  #[must_use]
  pub fn find_match(&self, path: &str, title: &str) -> Option<&SidebarMatch> {
    self.matches.iter().find(|m| m.matches(path, title))
  }
}

/// Ordering algorithm for sidebar items.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SidebarOrdering {
  /// Sort alphabetically by title.
  #[default]
  Alphabetical,
  /// Use custom ordering via position field.
  Custom,
  /// Preserve filesystem ordering.
  Filesystem,
}

/// Path matching criteria (exact or regex).
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct PathMatch {
  /// Exact path match.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exact: Option<String>,

  /// Regex pattern for path matching.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub regex: Option<String>,
}

impl<'de> Deserialize<'de> for PathMatch {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct PathMatchVisitor;

    impl<'de> Visitor<'de> for PathMatchVisitor {
      type Value = PathMatch;

      fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
      ) -> std::fmt::Result {
        formatter
          .write_str("a string or a map with 'exact' and/or 'regex' fields")
      }

      fn visit_str<E>(self, value: &str) -> Result<PathMatch, E>
      where
        E: de::Error,
      {
        // Shorthand: path = "foo" becomes path.exact = "foo"
        Ok(PathMatch {
          exact: Some(value.to_string()),
          regex: None,
        })
      }

      fn visit_map<M>(self, mut map: M) -> Result<PathMatch, M::Error>
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

        Ok(PathMatch { exact, regex })
      }
    }

    deserializer.deserialize_any(PathMatchVisitor)
  }
}

/// Title matching criteria (exact or regex).
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct TitleMatch {
  /// Exact title match.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exact: Option<String>,

  /// Regex pattern for title matching.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub regex: Option<String>,
}

impl<'de> Deserialize<'de> for TitleMatch {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct TitleMatchVisitor;

    impl<'de> Visitor<'de> for TitleMatchVisitor {
      type Value = TitleMatch;

      fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
      ) -> std::fmt::Result {
        formatter
          .write_str("a string or a map with 'exact' and/or 'regex' fields")
      }

      fn visit_str<E>(self, value: &str) -> Result<TitleMatch, E>
      where
        E: de::Error,
      {
        // Shorthand: title = "foo" becomes title.exact = "foo"
        Ok(TitleMatch {
          exact: Some(value.to_string()),
          regex: None,
        })
      }

      fn visit_map<M>(self, mut map: M) -> Result<TitleMatch, M::Error>
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

        Ok(TitleMatch { exact, regex })
      }
    }

    deserializer.deserialize_any(TitleMatchVisitor)
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
  /// Check if this rule matches the given path and title.
  ///
  /// All specified conditions must match (AND logic).
  #[must_use]
  pub fn matches(&self, path_str: &str, title_str: &str) -> bool {
    // Check path matching
    if let Some(ref path_match) = self.path {
      // Check exact path match
      if let Some(ref exact_path) = path_match.exact {
        if path_str != exact_path {
          return false;
        }
      }

      // Check path regex match
      if let Some(ref regex_path) = path_match.regex {
        if let Ok(re) = Regex::new(regex_path) {
          if !re.is_match(path_str) {
            return false;
          }
        } else {
          // Invalid regex means no match
          return false;
        }
      }
    }

    // Check title matching
    if let Some(ref title_match) = self.title {
      // Check exact title match
      if let Some(ref exact_title) = title_match.exact {
        if title_str != exact_title {
          return false;
        }
      }

      // Check title regex match
      if let Some(ref regex_title) = title_match.regex {
        if let Ok(re) = Regex::new(regex_title) {
          if !re.is_match(title_str) {
            return false;
          }
        } else {
          // Invalid regex means no match
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sidebar_ordering_deserialization() {
    #[derive(Deserialize)]
    struct Wrapper {
      ordering: SidebarOrdering,
    }

    let toml = r#"ordering = "alphabetical""#;
    let wrapper: Wrapper = toml::from_str(toml).unwrap();
    assert!(matches!(wrapper.ordering, SidebarOrdering::Alphabetical));

    let toml = r#"ordering = "custom""#;
    let wrapper: Wrapper = toml::from_str(toml).unwrap();
    assert!(matches!(wrapper.ordering, SidebarOrdering::Custom));

    let toml = r#"ordering = "filesystem""#;
    let wrapper: Wrapper = toml::from_str(toml).unwrap();
    assert!(matches!(wrapper.ordering, SidebarOrdering::Filesystem));
  }

  #[test]
  fn test_sidebar_config_default() {
    let config = SidebarConfig::default();
    assert!(!config.numbered);
    assert!(!config.number_special_files);
    assert!(matches!(config.ordering, SidebarOrdering::Alphabetical));
    assert!(config.matches.is_empty());
  }

  #[test]
  fn test_sidebar_match_exact_path() {
    let m = SidebarMatch {
      path:      Some(PathMatch {
        exact: Some("getting-started.md".to_string()),
        regex: None,
      }),
      title:     None,
      new_title: None,
      position:  Some(1),
    };

    assert!(m.matches("getting-started.md", "Any Title"));
    assert!(!m.matches("other.md", "Any Title"));
  }

  #[test]
  fn test_sidebar_match_regex_path() {
    let m = SidebarMatch {
      path:      Some(PathMatch {
        exact: None,
        regex: Some(r"^api/.*\.md$".to_string()),
      }),
      title:     None,
      new_title: None,
      position:  Some(50),
    };

    assert!(m.matches("api/foo.md", "Any Title"));
    assert!(m.matches("api/bar/baz.md", "Any Title"));
    assert!(!m.matches("other.md", "Any Title"));
  }

  #[test]
  fn test_sidebar_match_exact_title() {
    let m = SidebarMatch {
      path:      None,
      title:     Some(TitleMatch {
        exact: Some("Release Notes".to_string()),
        regex: None,
      }),
      new_title: Some("What's New".to_string()),
      position:  Some(999),
    };

    assert!(m.matches("any/path.md", "Release Notes"));
    assert!(!m.matches("any/path.md", "Other Title"));
  }

  #[test]
  fn test_sidebar_match_regex_title() {
    let m = SidebarMatch {
      path:      None,
      title:     Some(TitleMatch {
        exact: None,
        regex: Some(r"^Release.*".to_string()),
      }),
      new_title: Some("What's New".to_string()),
      position:  Some(999),
    };

    assert!(m.matches("any/path.md", "Release Notes"));
    assert!(m.matches("any/path.md", "Release 1.0"));
    assert!(!m.matches("any/path.md", "Other Title"));
  }

  #[test]
  fn test_sidebar_match_combined_conditions() {
    let m = SidebarMatch {
      path:      Some(PathMatch {
        exact: None,
        regex: Some(r"^api/.*\.md$".to_string()),
      }),
      title:     Some(TitleMatch {
        exact: None,
        regex: Some(r"^API.*".to_string()),
      }),
      new_title: None,
      position:  Some(50),
    };

    // Both conditions must match
    assert!(m.matches("api/foo.md", "API Reference"));
    assert!(!m.matches("api/foo.md", "Other Title"));
    assert!(!m.matches("other.md", "API Reference"));
  }

  #[test]
  fn test_sidebar_match_get_position() {
    let m = SidebarMatch {
      path:      Some(PathMatch {
        exact: Some("test.md".to_string()),
        regex: None,
      }),
      title:     None,
      new_title: None,
      position:  Some(42),
    };

    assert_eq!(m.get_position(), Some(42));
  }

  #[test]
  fn test_sidebar_match_get_title() {
    let m = SidebarMatch {
      path:      Some(PathMatch {
        exact: Some("test.md".to_string()),
        regex: None,
      }),
      title:     None,
      new_title: Some("Custom Title".to_string()),
      position:  None,
    };

    assert_eq!(m.get_title(), Some("Custom Title"));
  }

  #[test]
  fn test_sidebar_config_find_match() {
    let config = SidebarConfig {
      numbered:             true,
      number_special_files: false,
      ordering:             SidebarOrdering::Custom,
      matches:              vec![
        SidebarMatch {
          path:      Some(PathMatch {
            exact: Some("getting-started.md".to_string()),
            regex: None,
          }),
          title:     None,
          new_title: None,
          position:  Some(1),
        },
        SidebarMatch {
          path:      Some(PathMatch {
            exact: None,
            regex: Some(r"^api/.*\.md$".to_string()),
          }),
          title:     None,
          new_title: None,
          position:  Some(50),
        },
      ],
    };

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
      matches:              vec![
        SidebarMatch {
          path:      Some(PathMatch {
            exact: Some("test.md".to_string()),
            regex: None,
          }),
          title:     None,
          new_title: Some("First".to_string()),
          position:  Some(1),
        },
        SidebarMatch {
          path:      Some(PathMatch {
            exact: Some("test.md".to_string()),
            regex: None,
          }),
          title:     None,
          new_title: Some("Second".to_string()),
          position:  Some(2),
        },
      ],
    };

    let m = config.find_match("test.md", "Title").unwrap();
    assert_eq!(m.new_title.as_deref(), Some("First"));
    assert_eq!(m.position, Some(1));
  }

  #[test]
  fn test_sidebar_config_get_position() {
    let config = SidebarConfig {
      numbered:             false,
      number_special_files: false,
      ordering:             SidebarOrdering::Alphabetical,
      matches:              vec![SidebarMatch {
        path:      Some(PathMatch {
          exact: Some("test.md".to_string()),
          regex: None,
        }),
        title:     None,
        new_title: None,
        position:  Some(42),
      }],
    };

    assert_eq!(
      config
        .find_match("test.md", "Title")
        .and_then(|m| m.get_position()),
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
      matches:              vec![SidebarMatch {
        path:      Some(PathMatch {
          exact: Some("test.md".to_string()),
          regex: None,
        }),
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

    let config: SidebarConfig = toml::from_str(toml).unwrap();
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

    let config: SidebarConfig = serde_json::from_str(json).unwrap();
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

    let config: SidebarConfig =
      toml::from_str(&format!("[sidebar]\n{}", toml)).unwrap();
    assert_eq!(config.matches.len(), 1);

    // Shorthand string should become exact match
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.exact.as_deref()),
      Some("getting-started.md")
    );
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.regex.as_ref()),
      None
    );
  }

  #[test]
  fn test_title_match_shorthand_string() {
    let toml = r#"
[[matches]]
title = "Getting Started"
position = 1
"#;

    let config: SidebarConfig =
      toml::from_str(&format!("[sidebar]\n{}", toml)).unwrap();
    assert_eq!(config.matches.len(), 1);

    // Shorthand string should become exact match
    assert_eq!(
      config.matches[0]
        .title
        .as_ref()
        .and_then(|t| t.exact.as_deref()),
      Some("Getting Started")
    );
    assert_eq!(
      config.matches[0]
        .title
        .as_ref()
        .and_then(|t| t.regex.as_ref()),
      None
    );
  }

  #[test]
  fn test_mixed_shorthand_and_nested() {
    let toml = r#"
numbered = true

[[matches]]
path = "installation.md"
new_title = "Setup"
position = 1

[[matches]]
path.regex = "^api/.*\\.md$"
title = "API Reference"
position = 2
"#;

    let config: SidebarConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.matches.len(), 2);

    // First: path shorthand
    assert_eq!(
      config.matches[0]
        .path
        .as_ref()
        .and_then(|p| p.exact.as_deref()),
      Some("installation.md")
    );

    // Second: path.regex nested, title shorthand
    assert_eq!(
      config.matches[1]
        .path
        .as_ref()
        .and_then(|p| p.regex.as_deref()),
      Some(r"^api/.*\.md$")
    );
    assert_eq!(
      config.matches[1]
        .title
        .as_ref()
        .and_then(|t| t.exact.as_deref()),
      Some("API Reference")
    );
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

    let config: SidebarConfig = serde_json::from_str(json).unwrap();
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
}
