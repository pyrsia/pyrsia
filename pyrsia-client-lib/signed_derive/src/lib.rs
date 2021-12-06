//! This defines the derive(SignedStruct) macro. See the documentation for the Signed trait for documentation.

extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Field};

#[proc_macro_attribute]
pub fn signed_struct(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(args as AttributeArgs);
    let json_field_name = "_json0";
    let json_fields_named: syn::FieldsNamed =
        syn::parse2(quote!( #json_field_name : String ).into()).unwrap();
    let json_field: Field = json_fields_named.named.first().unwrap().to_owned();
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => fields.named.push(json_field),
                _ => (),
            }

            return quote! {
                        #ast
            impl<'a> ::pyrsia_client_lib::Signed<'a> for #(ast.ident)<'a> {
                    fn json(&self) -> Option<String> {
                        self._json.to_owned()
                    }

                    fn clear_json(&mut self) {
                        self._json = None;
                    }

                    fn set_json(&mut self, json: &str) {
                        self._json = Option::Some(json.to_string())
                    }
                };

                    }
            .into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
