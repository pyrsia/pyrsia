//! This defines the derive(SignedStruct) macro. See the documentation for the Signed trait for documentation.

extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use std::collections::HashSet;

use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Field, FieldsNamed};

#[proc_macro_attribute]
pub fn signed_struct(args: TokenStream, input: TokenStream) -> TokenStream {
    println!("parsing args");
    let _ = parse_macro_input!(args as AttributeArgs);
    println!("parsing input");
    let mut ast = parse_macro_input!(input as DeriveInput);
    let struct_ident = &ast.ident;
    println!("matching ast");
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            println!("data matches Struct");
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    println!("Struct contains named fields");
                    let json_field_name = unique_json_field_ident(fields);
                    println!("Field name is {}", json_field_name);
                    let json_field = construct_json_field(&json_field_name);
                    println!("Constructed field");
                    fields.named.push(json_field);
                    println!("generating output");
                    let output = quote! {
                    //#[derive(serde::Serialize, serde::Deserialize)]
                    #ast

                    // impl<'a> ::pyrsia_client_lib::Signed<'a> for #struct_ident<'a> {
                    //         pub fn json(&self) -> Option<String> {
                    //             self.#json_field_name.to_owned()
                    //         }
                    //
                    //         pub fn clear_json(&mut self) {
                    //             self.#json_field_name = None;
                    //         }
                    //
                    //         fn set_json(&mut self, json: &str) {
                    //             self.#json_field_name = Option::Some(json.to_string())
                    //         }
                    //     };
                    }
                    .into();
                    println!("Output: {}", output);
                    return output;
                }
                _ => {
                    return syn::parse::Error::new(
                        ast.span(),
                        "signed_struct may only be used with struct having named field.",
                    )
                    .to_compile_error()
                    .into()
                }
            }
        }
        _ => {
            return syn::parse::Error::new(ast.span(), "`add_field` has to be used with structs ")
                .to_compile_error()
                .into()
        }
    }
}

fn construct_json_field(field_name: &Ident) -> Field {
    let json_fields_named: syn::FieldsNamed = syn::parse2(
        quote!( {
            #[derivative::Der]
            #field_name : Option<String>
        } )
        .into(),
    )
    .unwrap();
    let json_field: Field = json_fields_named.named.first().unwrap().to_owned();
    json_field
}

fn unique_json_field_ident(fields: &FieldsNamed) -> Ident {
    let mut field_names: HashSet<String> = HashSet::new();
    for field in fields.named.iter() {
        for id in field.ident.iter() {
            field_names.insert(id.to_string());
        }
    }
    let mut counter = 0;
    loop {
        let mut candidate_name = String::from("_json");
        candidate_name.push_str(&counter.to_string());
        if !field_names.contains(candidate_name.as_str()) {
            return format_ident!("_json{}", counter.to_string());
        }
        counter += 1;
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
