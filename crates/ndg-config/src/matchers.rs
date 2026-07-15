use regex::Regex;
use serde::{
  Deserialize,
  Deserializer,
  Serialize,
  de::{self, MapAccess, Visitor},
};

pub(crate) trait MatchField: Sized {
  fn from_exact(exact: String) -> Self;
  fn from_parts(exact: Option<String>, regex: Option<String>) -> Self;
}

pub(crate) fn deserialize_match_field<'de, D, T>(
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
