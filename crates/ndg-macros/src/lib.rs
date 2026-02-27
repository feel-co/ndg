//! Proc-macros for NDG configuration system.
//!
//! Provides derive macros for automatic configuration handling.

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Data, DeriveInput, Fields, Type, parse_macro_input};

/// Attribute configuration for a field.
#[derive(Default)]
struct FieldConfig {
  /// The config key name (defaults to field name).
  key: Option<String>,

  /// Whether this field is nested.
  nested: bool,

  /// Deprecation info: (version, replacement).
  deprecated: Option<(String, Option<String>)>,

  /// Allow empty values (set to None).
  allow_empty: bool,

  /// Pending replacement value (set before deprecated).
  pending_replacement: Option<String>,
}

impl FieldConfig {
  fn from_attrs(attrs: &[Attribute]) -> Self {
    let mut config = Self::default();

    for attr in attrs {
      if !attr.path().is_ident("config") {
        continue;
      }

      let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("key") {
          let value = meta.value()?;
          let lit: syn::LitStr = value.parse()?;
          config.key = Some(lit.value());
        } else if meta.path.is_ident("deprecated") {
          let value = meta.value()?;
          let lit: syn::LitStr = value.parse()?;
          config.deprecated =
            Some((lit.value(), config.pending_replacement.take()));
        } else if meta.path.is_ident("replacement") {
          let value = meta.value()?;
          let lit: syn::LitStr = value.parse()?;
          config.pending_replacement = Some(lit.value());
        } else if meta.path.is_ident("allow_empty") {
          config.allow_empty = true;
        } else if meta.path.is_ident("nested") {
          config.nested = true;
        }
        Ok(())
      });
    }

    config
  }
}

/// Derive macro for configuration structs.
#[proc_macro_derive(Configurable, attributes(config))]
pub fn derive_configurable(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) =
    input.generics.split_for_impl();

  // Parse fields and generate override code
  let fields = match &input.data {
    Data::Struct(data) => &data.fields,
    _ => {
      return syn::Error::new_spanned(
        input,
        "Configurable can only be derived for structs",
      )
      .to_compile_error()
      .into();
    },
  };

  let field_handlers = generate_field_handlers(fields);
  let merge_handlers = generate_merge_handlers(fields);

  let expanded = quote! {
    impl #impl_generics #name #ty_generics #where_clause {
      /// Apply a configuration override by key.
      pub fn apply_override(
        &mut self,
        key: &str,
        value: &str,
      ) -> std::result::Result<(), crate::error::ConfigError> {
        use crate::error::ConfigError;

        #(#field_handlers)*

        Err(ConfigError::Config(format!(
          "Unknown configuration key: '{key}'. See documentation for supported keys.",
        )))
      }

      /// Merge another config into this one.
      pub fn merge_fields(&mut self, other: Self) {
        #(#merge_handlers)*
      }
    }
  };

  TokenStream::from(expanded)
}

fn generate_field_handlers(fields: &Fields) -> Vec<proc_macro2::TokenStream> {
  let mut handlers = Vec::new();

  for field in fields.iter() {
    let field_config = FieldConfig::from_attrs(&field.attrs);

    // Skip fields without config attributes unless they're nested configs
    let has_config_attr = field
      .attrs
      .iter()
      .any(|attr| attr.path().is_ident("config"));
    if !has_config_attr && !field_config.nested {
      continue;
    }

    let field_name = field.ident.as_ref().expect("Named field required");
    let field_key = field_config
      .key
      .clone()
      .unwrap_or_else(|| field_name.to_string());
    let field_type = &field.ty;

    // Generate the match arm for this field
    let handler =
      generate_field_handler(field_name, field_key, field_type, &field_config);
    handlers.push(handler);
  }

  handlers
}

fn generate_field_handler(
  field_name: &syn::Ident,
  field_key: String,
  field_type: &Type,
  config: &FieldConfig,
) -> proc_macro2::TokenStream {
  if config.nested {
    return generate_nested_handler(field_name, &field_key, field_type, config);
  }

  // Skip Vec and HashMap fields, they can't be directly overridden
  let type_str = field_type.to_token_stream().to_string();
  if type_str.starts_with("Vec<") || type_str.contains("HashMap<") {
    return quote! {};
  }

  // Handle deprecation
  let deprecation_check =
    if let Some((version, replacement)) = &config.deprecated {
      let msg = if let Some(replacement) = replacement {
        format!(
          "The '{}' config key is deprecated since {}. Use '{}' instead.",
          field_key, version, replacement
        )
      } else {
        format!(
          "The '{}' config key is deprecated since {}.",
          field_key, version
        )
      };
      quote! {
        log::warn!(#msg);
      }
    } else {
      quote! {}
    };

  // Generate type-specific parsing
  let value_assignment =
    generate_value_assignment(field_name, field_type, config);

  let assignment_expr = if config.deprecated.is_some() {
    quote! {
      #[allow(deprecated)]
      { #value_assignment }
    }
  } else {
    value_assignment
  };

  quote! {
    #deprecation_check
    if key == #field_key {
      #assignment_expr
      return Ok(());
    }
  }
}

fn generate_nested_handler(
  field_name: &syn::Ident,
  base_key: &str,
  _field_type: &Type,
  _config: &FieldConfig,
) -> proc_macro2::TokenStream {
  // For nested configs, we need to handle sub-keys like "search.enable"
  let base_key_str = base_key.to_string();

  quote! {
    if key.starts_with(concat!(#base_key_str, ".")) {
      if self.#field_name.is_none() {
        self.#field_name = Some(Default::default());
      }
      if let Some(ref mut inner) = self.#field_name {
        let subkey = &key[(#base_key_str.len() + 1)..];
        return inner.apply_override(subkey, value);
      }
    }
  }
}

fn generate_value_assignment(
  field_name: &syn::Ident,
  field_type: &Type,
  config: &FieldConfig,
) -> proc_macro2::TokenStream {
  let type_str = field_type.to_token_stream().to_string();

  // Option<T> handling
  if type_str.starts_with("Option ") || type_str.starts_with("Option<") {
    if config.allow_empty {
      quote! {
        self.#field_name = if value.is_empty() {
          None
        } else {
          Some(value.parse().map_err(|_| ConfigError::Config(
            format!("Invalid value for '{}': '{}'", stringify!(#field_name), value)
          ))?)
        };
      }
    } else {
      quote! {
        self.#field_name = Some(value.parse().map_err(|_| ConfigError::Config(
          format!("Invalid value for '{}': '{}'", stringify!(#field_name), value)
        ))?);
      }
    }
  }
  // PathBuf handling
  else if type_str.contains("PathBuf") {
    quote! {
      self.#field_name = std::path::PathBuf::from(value);
    }
  }
  // String handling
  else if type_str == "String" {
    quote! {
      self.#field_name = value.to_string();
    }
  }
  // Bool handling
  else if type_str == "bool" {
    quote! {
      self.#field_name = match value.to_lowercase().as_str() {
        "true" | "yes" | "1" => true,
        "false" | "no" | "0" => false,
        _ => {
          return Err(ConfigError::Config(format!(
            "Invalid boolean value for '{}': '{}'. Expected true/false, yes/no, or 1/0",
            stringify!(#field_name), value
          )));
        }
      };
    }
  }
  // SidebarOrdering handling
  else if type_str == "SidebarOrdering" {
    quote! {
      self.#field_name = value.parse().map_err(|e: String| ConfigError::Config(format!(
        "Invalid value for '{}': '{}' - {}",
        stringify!(#field_name), value, e
      )))?;
    }
  }
  // Option<SidebarOrdering> handling
  else if type_str.contains("Option<SidebarOrdering>")
    || type_str.contains("Option < SidebarOrdering >")
  {
    quote! {
      self.#field_name = if value.is_empty() {
        None
      } else {
        Some(value.parse().map_err(|e: String| ConfigError::Config(format!(
          "Invalid value for '{}': '{}' - {}",
          stringify!(#field_name), value, e
        )))?)
      };
    }
  }
  // usize handling
  else if type_str == "usize" {
    quote! {
      self.#field_name = value.parse().map_err(|_| ConfigError::Config(format!(
        "Invalid value for '{}': '{}'. Expected a positive integer",
        stringify!(#field_name), value
      )))?;
    }
  }
  // Option<usize> handling
  else if type_str.contains("Option<usize>") {
    quote! {
      self.#field_name = if value.is_empty() {
        None
      } else {
        Some(value.parse().map_err(|_| ConfigError::Config(format!(
          "Invalid value for '{}': '{}'. Expected a positive integer",
          stringify!(#field_name), value
        )))?)
      };
    }
  }
  // u8 handling
  else if type_str == "u8" {
    quote! {
      self.#field_name = value.parse().map_err(|_| ConfigError::Config(format!(
        "Invalid value for '{}': '{}'. Expected a number between 0-255",
        stringify!(#field_name), value
      )))?;
    }
  }
  // Option<u8> handling
  else if type_str.contains("Option<u8>") {
    quote! {
      self.#field_name = if value.is_empty() {
        None
      } else {
        Some(value.parse().map_err(|_| ConfigError::Config(format!(
          "Invalid value for '{}': '{}'. Expected a number between 0-255",
          stringify!(#field_name), value
        )))?)
      };
    }
  }
  // f32 handling
  else if type_str == "f32" {
    quote! {
      self.#field_name = value.parse().map_err(|_| ConfigError::Config(format!(
        "Invalid value for '{}': '{}'. Expected a number",
        stringify!(#field_name), value
      )))?;
    }
  }
  // Option<f32> handling
  else if type_str.contains("Option<f32>") {
    quote! {
      self.#field_name = if value.is_empty() {
        None
      } else {
        Some(value.parse().map_err(|_| ConfigError::Config(format!(
          "Invalid value for '{}': '{}'. Expected a number",
          stringify!(#field_name), value
        )))?)
      };
    }
  }
  // Default: try to parse
  else {
    quote! {
      self.#field_name = value.parse().map_err(|_| ConfigError::Config(format!(
        "Invalid value for '{}': '{}'",
        stringify!(#field_name), value
      )))?;
    }
  }
}

fn generate_merge_handlers(fields: &Fields) -> Vec<proc_macro2::TokenStream> {
  let mut handlers = Vec::new();

  for field in fields.iter() {
    let field_config = FieldConfig::from_attrs(&field.attrs);
    let field_name = field.ident.as_ref().expect("Named field required");
    let field_type = &field.ty;
    let type_str = field_type.to_token_stream().to_string();

    let handler = if field_config.nested {
      // For nested configs, replace if other has Some
      quote! {
        if other.#field_name.is_some() {
          self.#field_name = other.#field_name;
        }
      }
    } else if type_str.starts_with("Option<HashMap<")
      || type_str.starts_with("Option < HashMap <")
    {
      // Option<HashMap> fields: extend if both Some, otherwise replace only if
      // other is Some
      quote! {
        match (self.#field_name.as_mut(), other.#field_name) {
          (Some(self_map), Some(other_map)) => {
            self_map.extend(other_map);
          }
          (None, Some(other_map)) => {
            self.#field_name = Some(other_map);
          }
          _ => {}
        }
      }
    } else if type_str.starts_with("Option<")
      || type_str.starts_with("Option <")
    {
      // Option fields: replace if other has Some
      quote! {
        if other.#field_name.is_some() {
          self.#field_name = other.#field_name;
        }
      }
    } else if type_str.starts_with("Vec<") || type_str.starts_with("Vec <") {
      // Vec fields: extend
      quote! {
        self.#field_name.extend(other.#field_name);
      }
    } else if type_str.contains("HashMap") {
      // HashMap fields: extend (other takes precedence)
      quote! {
        self.#field_name.extend(other.#field_name);
      }
    } else {
      // Plain fields: always replace
      quote! {
        self.#field_name = other.#field_name;
      }
    };

    handlers.push(handler);
  }

  handlers
}
