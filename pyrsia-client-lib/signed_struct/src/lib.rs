//! This defines the derive(SignedStruct) macro. See the documentation for the Signed trait for documentation.

extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use std::collections::HashSet;

use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Field, FieldsNamed, Visibility, TypeReference};

#[proc_macro_attribute]
pub fn signed_struct(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(args as AttributeArgs);
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => match &mut struct_data.fields {
            syn::Fields::Named(fields) => {
                match unique_json_field_ident(fields) {
                    Ok(json_field_name) => {
                        let json_field = construct_json_field(&json_field_name);
                        fields.named.push(json_field);
                    }
                    Err(error) => return error.to_compile_error().into(),
                }
                println!("generating output");
                let output = quote! {
                #[derive(serde::Serialize, serde::Deserialize)]
                #[derive(signed_struct::SignedStructDerive)]
                #ast
                }
                .into();
                println!("Output for signed_struct: {}", output);
                output
            }
            _ => syn::parse::Error::new(
                ast.span(),
                "signed_struct may only be used with structs having named fields.",
            )
            .to_compile_error()
            .into(),
        },
        _ => syn::parse::Error::new(ast.span(), "signed_struct may only be used with structs ")
            .to_compile_error()
            .into(),
    }
}

fn construct_json_field(field_name: &Ident) -> Field {
    let json_fields_named: syn::FieldsNamed = syn::parse2(
        quote!( {
            #[serde(skip)]
            #field_name : Option<String>
        } )
        .into(),
    )
    .unwrap();
    let json_field: Field = json_fields_named.named.first().unwrap().to_owned();
    json_field
}

fn unique_json_field_ident(fields: &FieldsNamed) -> Result<Ident, syn::parse::Error> {
    let mut field_names: HashSet<String> = HashSet::new();
    for field in fields.named.iter() {
        if field.vis != Visibility::Inherited {
            return Err(syn::parse::Error::new(
                field.span(),
                "signed_struct requires all fields to be private",
            ));
        }
        for id in field.ident.iter() {
            field_names.insert(id.to_string());
        }
    }
    let mut counter = 0;
    loop {
        let mut candidate_name = String::from("_json");
        candidate_name.push_str(&counter.to_string());
        if !field_names.contains(candidate_name.as_str()) {
            return Ok(format_ident!("_json{}", counter.to_string()));
        }
        counter += 1;
    }
}

#[proc_macro_derive(SignedStructDerive)]
pub fn signed_struct_derive(input: TokenStream) -> TokenStream {
    println!("parsing input");
    let ast = parse_macro_input!(input as DeriveInput);
    match &ast.data {
        syn::Data::Struct(ref struct_data) => {
            println!("data matches Struct");
            match &struct_data.fields {
                syn::Fields::Named(fields) => {
                    println!("Struct contains named fields");
                    match scan_fields(fields) {
                        Ok((json_field_name, fields_vec, field_ident_vec)) => {
                            println!("generating output from signed_struct_derive");
                            let struct_ident = &ast.ident;
                            let output = quote! {
                                impl<'π> ::pyrsia_client_lib::signed::Signed<'π> for #struct_ident<'π> {
                                    fn json(&self) -> Option<String> {
                                        self.#json_field_name.to_owned()
                                    }

                                    fn clear_json(&mut self) {
                                        self.#json_field_name = None;
                                    }

                                    fn set_json(&mut self, json: &str) {
                                        self.#json_field_name = Option::Some(json.to_string())
                                    }
                                }

                                impl #struct_ident<'_> {
                                    fn new(#(#fields_vec),*) -> #struct_ident {
                                        #struct_ident{ #(#field_ident_vec),* , #json_field_name: None }
                                    }
                                }
                            }
                            .into();
                            println!("Output from signed_struct_derive: {}", output);
                            return output;
                        }
                        Err(error) => error.to_compile_error().into(),
                    }
                }
                _ => {
                    return syn::parse::Error::new(
                        ast.span(),
                        "signed_struct_derive may only be used with structs having named fields.",
                    )
                    .to_compile_error()
                    .into()
                }
            }
        }
        _ => {
            return syn::parse::Error::new(
                ast.span(),
                "signed_struct_derive may only be used with structs ",
            )
            .to_compile_error()
            .into()
        }
    }
}

fn scan_fields(fields: &FieldsNamed) -> Result<(Ident, Vec<Field>, Vec<Ident>), syn::parse::Error> {
    let mut fields_vec = Vec::new();
    let mut field_name_vec: Vec<Ident> = Vec::new();
    for field in fields.named.iter() {
        let param_field = remove_attrs_and_lifetime(field);
        fields_vec.push(param_field.clone());
        field_name_vec.push(param_field.ident.clone().unwrap())
    }

    match fields_vec.pop() {
        Some(_) => Ok((field_name_vec.pop().unwrap(), fields_vec, field_name_vec)),
        None => Err(syn::parse::Error::new(
            fields.span(),
            "signed_struct_derive does not work with an empty struct",
        )),
    }
}

fn remove_attrs_and_lifetime(field: &Field) -> Field {
    let mut param_field = field.clone();
    param_field.attrs = Vec::new();
    if let syn::Type::Reference(t) = param_field.ty {
        param_field.ty = syn::Type::Reference(TypeReference { lifetime: None, ..t });
    }
    param_field
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
