/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//!
//! The signed JSON associated with a struct can be stored or sent in a message as a representation
//! of the contents of the struct. The signatures can be validated to verify that the contents of
//! the JSON have not been modified since the JSON was signed. Multiple signatures can be applied to
//! a signed struct.
//!
//! The first thing that you will do with this module is to use its `create_key_pair` function to
//! create a key pair. You will use the private key in the key pair to sign structs. The public key
//! is used to identify the signer.
//! ```
//! // Use adjecent crate (as dictated by Rust)
//! use signed::signed::{SignatureKeyPair, create_key_pair, JwsSignatureAlgorithms};
//! let key_pair: SignatureKeyPair = create_key_pair(JwsSignatureAlgorithms::RS512).unwrap();
//! ```
//!
//! The next thing to do is define some signed structs. Signed structs implement the `Signed` trait.
//! However, it is not recommended that you implement the `Signed` trait directly. Instead, you
//! should annotate the struct like this <br>
//! `   #[signed_struct]` <br>
//! `   struct Foo<'a> {` <br>
//! `       foo: String,` <br>
//! `       bar: u32,` <br>
//! `       zot: &'a str,` <br>
//! `   }` <br>
//!
//! This annotation runs a macro that add some fields to support the Signed trait, implements the
//! signed trait, and generates getters and setters for the struct. There is not a full example of
//! its use here to avoid Cargo complaining about a circular dependency. You can see a detailed
//! example in the source for `signed_struct/tests/lib.rs'. This is the best example of how to use
//! signed struct. You should read it.
//!
//! Getters are generated with the signature `fn field(&self) -> &type`.
//!
//! Setters are generated as `fn field(&mut self, val: type)`. In addition to setting their field,
//! the setters also call the `clear_json()` method provided by the `Signed` trait. This removes
//! any JSON currently associated with the struct because it is no longer valid after the struct's
//! field has been modified.
//!
//! You should not create instances of the struct directly. Instead, you should use the generated
//! `new` method. To create an instance of the `Foo` struct shown above, you could write something
//! like this: <br>
//! `let foo = Foo::new(foo_value, bar_value, zot_value);`
//!
//! It is recommended that signed structs be defined in a separate module that contains just the
//! signed struct. This is so that nothing but the generated getters and setters can access the
//! struct's fields.  Note that signed structs are not allowed to have public fields.


/// This test shows how to use the <br>
/// `#[signed_struct]`<br>
/// macro
#[cfg(test)]
mod tests {
    extern crate anyhow;
    extern crate derive_builder;
    extern crate pyrsia_client_lib;
    extern crate serde;
    extern crate signed;

    use log::{debug, info};
    use signed::signed::{
        create_key_pair, Attestation, JwsSignatureAlgorithms, SignatureKeyPair, Signed,
    };

    // Create a signed struct in its own module to prevent direct access to its fields. They will
    // be accessed through the generated getters and setters.
    pub mod foo {
        // These always need to be included in the same scope the macro is bing used
        use signed::signed::Signed;
        use signed_struct::signed_struct;

        #[signed_struct]
        #[derive(Debug)]
        pub struct Foo<'a> {
            foo: String,
            bar: u32,
            zot: &'a str,
            //zing: Option<u64>, // All option types are defaulted to None if the builder is not given a value.
            // Putting a default makes it optional to set this in the builder.
            // #[builder(default = "Vec::new()")]
            // gonkulators: Vec<u128>,
        }
    }
    use foo::*;

    #[test]
    fn test_generated_methods() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();

        info!("Testing happy case");

        // Create a signature pair for signing JSON.
        // Values to use for populating our first signed struct.
        let key_pair: SignatureKeyPair = create_key_pair(JwsSignatureAlgorithms::RS512)?;

        let foo_value: String = String::from("π is 16 bit unicode");
        let foo_value_clone = foo_value.clone();
        let bar_value: u32 = 23894;

        let zot_value: &str = "🦽is 32 bit unicode";
        //Test methods of Foo generated by [signed_struct]
        let mut foo: Foo = FooBuilder::default()
            .foo(foo_value)
            .bar(bar_value)
            .zot(zot_value)
            .build()?;
        debug!("Initial content of struct is {:?}", foo);

        foo.sign_json(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )?;
        // Since we have not modified the contents of the struct, its signature should verify
        // successfully.
        foo.verify_signature()?;
        assert_eq!(foo_value_clone, *foo.foo()); // The * is needed because the getters add a & to the type.
        assert_eq!(bar_value, *foo.bar());
        assert_eq!(zot_value, *foo.zot());

        // after signing, there should be json.
        assert!(foo.json().is_some());

        info!("Signing a second time to see that the struct can have two valid signatures");
        foo.sign_json(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )?;
        let attestations: Vec<Attestation> = foo.verify_signature()?;
        assert!(attestations
            .iter()
            .all(|attestation| attestation.signature_is_valid()));

        info!("Modifying struct to verify that it is unsigned after modification.");
        // Now we are going to exercise the generated setters, which should have the side effect of
        // clearing the signed JSON.
        let foo_value: String = String::from("π is 16 bit unicode");
        let foo_value_clone = foo_value.clone();
        let bar_value: u32 = 736;
        let zot_value: &str = "asdf";

        foo.set_foo(foo_value);
        foo.set_bar(bar_value);
        foo.set_zot(zot_value);
        debug!("After modification, content of struct is {:?}", foo);

        assert_eq!(foo_value_clone, *foo.foo()); // The * is needed because the getters add a & to the type.
        assert_eq!(bar_value, *foo.bar());
        assert_eq!(zot_value, *foo.zot());

        // after previous set calls there should be no JSON.
        assert!(foo.json().is_none());

        info!("Sign the now unsigned JSON and verify the signature");
        // Create new JSON by signing.
        foo.sign_json(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )?;

        // after signing, there should be json.
        assert!(foo.json().is_some());
        foo.verify_signature()?;

        let json: &str = &foo.json().unwrap();
        println!("JSON: {}", json);

        info!("Create a copy of the struct from the signed JSON and then verify the copy's signature.");
        // Create a copy of the first instance of `Foo` from its signed JSON
        let foo2: Foo = Foo::from_json_string(json)?;

        assert_eq!(foo2.json().unwrap(), json);

        // after being created from json the signature should be valid and we can examine
        // information about the signature.
        let attestations2: Vec<Attestation> = foo2.verify_signature()?;

        // We just signed it once.
        assert_eq!(attestations2.len(), 1);

        // Check that the signature information is as expected.
        let attestation = &attestations[0];
        assert!(attestation.signature_is_valid());
        assert!(attestation.signature_algorithm().is_some());
        assert_eq!(
            &JwsSignatureAlgorithms::RS512,
            &attestation.signature_algorithm().unwrap()
        );
        assert!(attestation.expiration_time().is_none());
        assert!(attestation.timestamp().is_some());
        assert!(attestation.public_key().is_some());
        assert_eq!(
            key_pair.public_key,
            attestation.public_key().clone().unwrap()
        );
        Ok(())
    }
}
