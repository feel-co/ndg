use proc_macro2::TokenStream;
use quote::quote;
use syn::{
  Data,
  DeriveInput,
  Expr,
  Fields,
  LitStr,
  Meta,
  Token,
  punctuated::Punctuated,
};

use crate::{first_type_arg, type_is};

pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
  let Data::Struct(data) = &input.data else {
    return Err(syn::Error::new_spanned(
      input,
      "ConfigTemplate requires a struct",
    ));
  };
  let Fields::Named(fields) = &data.fields else {
    return Err(syn::Error::new_spanned(
      input,
      "ConfigTemplate requires named fields",
    ));
  };
  let mut values = Vec::new();
  let mut sections = Vec::new();

  for field in &fields.named {
    let Some(name) = &field.ident else { continue };
    let mut key = name.to_string();
    let mut skip = false;
    let mut nested = false;
    let mut example: Option<Expr> = None;
    let mut docs = Vec::new();
    for attr in &field.attrs {
      if attr.path().is_ident("doc") {
        if let Meta::NameValue(meta) = &attr.meta
          && let Expr::Lit(expr) = &meta.value
          && let syn::Lit::Str(text) = &expr.lit
        {
          docs.push(text.value().trim().to_string());
        }
      } else if attr.path().is_ident("template") {
        attr.parse_nested_meta(|meta| {
          if meta.path.is_ident("nested") {
            nested = true;
          } else if meta.path.is_ident("example") {
            example = Some(meta.value()?.parse()?);
          } else {
            return Err(meta.error("expected nested or example"));
          }
          Ok(())
        })?;
      } else if attr.path().is_ident("serde") || attr.path().is_ident("config")
      {
        let attributes = attr
          .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in attributes {
          if attr.path().is_ident("serde") {
            if meta.path().is_ident("skip")
              || meta.path().is_ident("skip_serializing")
            {
              skip = true;
            } else if meta.path().is_ident("rename") {
              key = renamed_key(&meta)?;
            }
          } else if meta.path().is_ident("nested") {
            nested = true;
          }
        }
      }
    }
    if skip {
      continue;
    }
    let docs = docs.join("\n");
    let ty = &field.ty;
    let optional = type_is(ty, "Option");
    let array = nested && type_is(ty, "Vec");
    let value_ty = if optional || array {
      first_type_arg(ty)
        .ok_or_else(|| syn::Error::new_spanned(ty, "expected a value type"))?
    } else {
      ty
    };
    let fallback = example.as_ref().map_or_else(
      || quote!(<#value_ty as Default>::default()),
      |expr| quote!(#expr),
    );
    let method = if nested {
      quote!(section)
    } else {
      quote!(field)
    };
    let extra = nested.then(|| quote!(, #array));
    let handler = if optional {
      quote! {
        if let Some(value) = &self.#name {
          writer.#method(#key, #docs, value, false #extra)?;
        } else {
          writer.#method(#key, #docs, &#fallback, true #extra)?;
        }
      }
    } else if array {
      quote! {
        if self.#name.is_empty() {
          writer.section(#key, #docs, &#fallback, true, true)?;
        } else {
          for value in &self.#name {
            writer.section(#key, #docs, value, false, true)?;
          }
        }
      }
    } else if !nested && type_is(ty, "Vec") && example.is_some() {
      quote! {
        if self.#name.is_empty() {
          writer.field(#key, #docs, &#fallback, true)?;
        } else {
          writer.field(#key, #docs, &self.#name, false)?;
        }
      }
    } else {
      quote! { writer.#method(#key, #docs, &self.#name, false #extra)?; }
    };
    if nested {
      sections.push(handler);
    } else {
      values.push(handler);
    }
  }

  let name = &input.ident;
  let (impl_generics, ty_generics, where_clause) =
    input.generics.split_for_impl();
  Ok(quote! {
    impl #impl_generics crate::templates::ConfigTemplate for #name #ty_generics #where_clause {
      fn write_template(&self, writer: &mut crate::templates::TemplateWriter) -> Result<(), crate::templates::TemplateError> {
        #(#values)*
        #(#sections)*
        Ok(())
      }
    }
  })
}

fn renamed_key(meta: &Meta) -> syn::Result<String> {
  let Meta::NameValue(rename) = meta else {
    return Err(syn::Error::new_spanned(meta, "expected rename = \"key\""));
  };
  let value = &rename.value;
  let lit: LitStr = syn::parse2(quote!(#value))?;
  Ok(lit.value())
}
