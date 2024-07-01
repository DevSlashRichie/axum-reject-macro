use std::fmt::format;

use darling::ast::{self, Fields};
use darling::{util, FromDeriveInput, FromVariant};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Ident, Type};

#[derive(Debug, FromVariant)]
#[darling(attributes(http_error))]
struct HttpErrorVariant {
    ident: Ident,
    fields: Fields<Type>,
    status: u16,
    message: String,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(http_error))]
struct HttpError {
    ident: Ident,
    generics: syn::Generics,
    data: ast::Data<HttpErrorVariant, util::Ignored>,
}

fn replace_first_occurrence(original: &str, target: &str, replacement: &str) -> String {
    if let Some(index) = original.find(target) {
        let (before, after) = original.split_at(index);
        let after_target = &after[target.len()..];
        return format!("{}{}{}", before, replacement, after_target);
    }
    original.to_string()
}

impl ToTokens for HttpError {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let generics = &self.generics;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let match_arms = self.data.as_ref()
            .take_enum()
            .expect("Should be an enum")
            .into_iter()
            .map(|variant| {
                let ident = &variant.ident;
                let status = variant.status;
                let message = &variant.message;

                let ph_count = message.matches("{}").count();

                let field = variant.fields.iter().enumerate().fold(Vec::new(), |mut acc, (next_i, _next_t)| {
                    if ph_count == 0 {
                        acc.push(quote! {
                            _
                        });
                    }

                    if ph_count != next_i {
                        let field_name = format_ident!("f_{}", next_i);

                        acc.push(quote! {
                            #field_name
                        });
                    }

                    acc
                });

                if field.is_empty() {
                    quote! {
                        Self::#ident => (axum::http::StatusCode::from_u16(#status).unwrap(), format!(r#"{{"error": "{}"}}"#, #message).to_string()).into_response()
                    }
                }  else if ph_count == 0 {
                    quote! {
                        Self::#ident(#(#field),*) => (axum::http::StatusCode::from_u16(#status).unwrap(), format!(r#"{{"error": "{}"}}"#, #message).to_string()).into_response()
                    }
                } else {
                    quote! {
                        Self::#ident(#(#field),*) => {
                                let new_message = {
                                let mut msg = #message.to_string();
                                #(
                                    msg = msg.replacen("{}", &#field.to_string(), 1);
                                )*
                                msg
                            };

                            (axum::http::StatusCode::from_u16(#status).unwrap(), format!(r#"{{"error": "{}"}}"#, new_message).to_string()).into_response()
                        }

                    }

                }
            });

        tokens.extend(quote! {
            #[automatically_derived]
            impl #impl_generics axum::response::IntoResponse for #ident #ty_generics #where_clause {
                fn into_response(self) -> axum::response::Response {
                    match self {
                        #(#match_arms),*
                    }
                }
            }
        });
    }
}

#[proc_macro_derive(HttpError, attributes(http_error))]
pub fn derive_http_error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let http_error = HttpError::from_derive_input(&input).unwrap();
    http_error.into_token_stream().into()
}
