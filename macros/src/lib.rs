use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_derive(SerdeIntoString, attributes(rename_all))]
pub fn serde_into_string(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);

  let ident = &input.ident;

  let has_serde_attr = input.attrs.iter().any(|attr| attr.path().is_ident("serde"));

  if !has_serde_attr {
    return syn::Error::new(
      ident.span(),
      "SerdeIntoString requires a #[serde(...)] attribute on the type",
    )
    .into_compile_error()
    .into();
  }

  quote! {
      impl From<String> for #ident {
          fn from(status: String) -> Self {
              match serde_json::from_str(&status) {
                  Ok(v) => v,
                  Err(e) => {
                      tracing::info!("Failed to parse {}: {}", stringify!(#ident), status);
                      panic!("{}", e);
                  }
              }
          }
      }

      impl From<#ident> for String {
          fn from(val: #ident) -> Self {
              serde_json::to_string(&val).unwrap()
          }
      }
  }
  .into()
}
