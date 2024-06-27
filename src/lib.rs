use darling::ast::{self, Fields};
use darling::{util, FromDeriveInput, FromVariant};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
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
                let field = variant.fields.iter().map(|_| quote! { _ }).collect::<Vec<_>>();

                if field.is_empty() {
                    quote! {
                        Self::#ident => (axum::http::StatusCode::from_u16(#status).unwrap(), format!(r#"{{"error": "{}"}}"#, #message).to_string()).into_response()
                    }
                } else {
                    quote! {
                        Self::#ident(#(#field),*) => (axum::http::StatusCode::from_u16(#status).unwrap(), format!(r#"{{"error": "{}"}}"#, #message).to_string()).into_response()
                    }
                }
            });

        tokens.extend(quote! {
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
