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

//! This file contains the unit tests for Pyrsia's mechanism for creating signed JSON and validating
//! the signatures. The idea is that signed JSON can be generated from struct's annotated with<br>
//! `#[signed_struct]`<br>
//! It is also possible to create these signed structs from the signed JSON

/// This test shows how to use the <br>
/// `#[signed_struct]`<br>
/// macro
#[cfg(test)]
mod tests {
    extern crate anyhow;
    extern crate derive_builder;
    extern crate serde;
    extern crate signed;

    // It is recommended that you defined a signed struct in its own module to prevent direct access
    // to its fields. They will be accessed through generated getters and setters.
    pub mod namespace {
        use pyrsia::model::package_type::PackageTypeName;
        // These always need to be included in the same scope as the #[signed_struct]
        use signed::signed::Signed;
        use signed_struct::signed_struct;

        #[signed_struct] // The #[signed_struct] should come before any derive macros.
        #[derive(Debug, PartialEq)] // Most structs will need to derive some trait implementations
        pub struct Namespace {
            id: String,
            package_type: PackageTypeName,
            namespace_path: Vec<String>,
            #[builder(default = "vec![]")]
            // Specifying a default means that the builder considers this field optional.
            administrators: Vec<Vec<u8>>,
            creation_time: Option<String>, // the builder automatically considers fields with an Option type to be optional
            modification_time: Option<String>,
        }
    }
    use log::{debug, info};
    use namespace::*;
    use pyrsia::model::package_type::PackageTypeName;
    use signed::signed::{
        create_key_pair, now_as_iso8601_string, Attestation, JwsSignatureAlgorithms,
        SignatureKeyPair, Signed,
    };

    #[test]
    fn test_signed_struct() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();

        info!("Testing happy case");

        // Create a signature pair for signing JSON.
        // Values to use for populating our first signed struct.
        let key_pair: SignatureKeyPair = create_key_pair(JwsSignatureAlgorithms::RS512)?;

        // Create an instance of the signed struct Namespace
        let mut namespace: Namespace = NamespaceBuilder::default()
            .id("65dfd245-666f-4352-8878-636f1812398c".to_string())
            .package_type(PackageTypeName::Docker)
            .namespace_path(vec!["docker".to_string()])
            .creation_time("2022-01-09T12:34:47.837Z")
            .build()?;

        debug!("Initial content of namespace struct is {:?}", namespace);

        // Sign the struct to generate the signed JSON
        namespace.sign_json(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )?;

        // Since we have not modified the contents of the struct, its signature should verify
        // successfully.
        namespace.verify_signature()?;

        // Get the contents of the struct with getters that are generated for you. The getter names are the same as the field names
        assert_eq!("65dfd245-666f-4352-8878-636f1812398c", *namespace.id()); // The * is needed because the getters add a & to the type.
        assert_eq!(PackageTypeName::Docker, *namespace.package_type());
        assert!(namespace.administrators().is_empty()); // this should have its default value of an empty vector

        // after signing, there should be json.
        assert!(namespace.json().is_some());
        info!(
            "This signed JSON string is associated with the struct: {}",
            namespace.json().unwrap()
        );

        // Now we exercise the generated setters, which has the side effect of clearing the signed JSON. The setter names are prefixed with `set_`
        namespace.set_modification_time(Some(now_as_iso8601_string())); // You don't need to wrap values for Option fields in Some. The setter handles that.
        debug!("After modification, content of struct is {:?}", namespace);

        // Because the struct was just modified, is has no JSON and so no signature.
        assert!(namespace.json().is_none());
        assert!(namespace.verify_signature().is_err());

        // We can now sign the struct again which also creates new JSON
        namespace.sign_json(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )?;

        // after signing, there should be json with a valid signature.
        assert!(namespace.json().is_some());
        namespace.verify_signature()?;

        let json: &str = &namespace.json().unwrap();
        info!("JSON after second signing: {}", json);

        info!("Create a copy of the struct from the signed JSON and then verify the copy's signature.");
        // Create a copy of the instance of `Namespace` from its signed JSON
        let json: String = namespace.json().unwrap();
        let namespace2: Namespace = Namespace::from_json_string(&json)?;

        assert_eq!(namespace2, namespace);

        // after being created from json the signature should be valid and we can examine
        // information about the signature.
        let attestations: Vec<Attestation> = namespace2.verify_signature()?;

        // We just signed it once (multiple signatures are not supported yet).
        assert_eq!(attestations.len(), 1);

        // Check that the signature information is as expected.
        let attestation = &attestations[0];
        assert!(attestation.signature_is_valid());
        assert_eq!(
            attestation.signature_algorithm().unwrap(),
            JwsSignatureAlgorithms::RS512
        );
        assert!(attestation.timestamp().is_some());
        assert_eq!(
            key_pair.public_key,
            attestation.public_key().clone().unwrap()
        );
        Ok(())
    }
}
