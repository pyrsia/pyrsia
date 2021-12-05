// This defines the SignedStruct macro. See the documentation for the Signed trait for documentation.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(SignedStruct)]
pub fn signed_struct_derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let output = quote! {
    impl<'a> Signed<'a> for #ident<'a> {
            fn json(&self) -> Option<String> {
                self._json.to_owned()
            }

            fn clear_json(&mut self) {
                self._json = None;
            }

            fn set_json(&mut self, json: &str) {
                self._json = Option::Some(json.to_string())
            }
        }
        };
    output.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
