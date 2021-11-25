extern crate anyhow;
extern crate ring;
extern crate serde;
extern crate serde_json;

use std::option::Option;

use anyhow::Result;
use ring::signature::RsaKeyPair;
use serde::{Deserialize, Serialize};

/// This trait should be implemented by all structs that contain signed data. Structs that implement
/// this trait should be annotated with
/// `#[derive(Serialize, Deserialize)]`
///
/// Pyrsia needs to manage a variety of data related to software artifacts. It will store all of
/// this data as JSON.  The reason for using JSON is to promote interoperability. If Pyrsia is
/// successful people will write their own implementations of Pyrsia nodes. For this reason, we
/// choose standard [JSON](https://www.json.org/json-en.html) .
///
/// All JSON that Pyrsia manages must be signed so that we can attribute it to a source and be
/// confident that it has not been modified since it was signed. Because we are using JSON based
/// signatures, when we deserialize JSON to a struct, to be considered signed, the struct must
/// contain a reference to the JSON it was deserialized from, so we can still verify the signature.
///
/// Methods that modify the contents of a signed struct should discard its associated JSON by
/// calling the clear_json method, since the JSON no longer matches that struct.
///
/// Given the above description of the purposes of the `Signed` trait, the descriptions of its
/// methods should be understood in this context.
///
/// It is recommended for consistency that structs that implement this trait are declared
/// like this with a field named `__json` to refer to the struct's json string:
/// ```
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Foo<'a> {
///   foo: &'a str,
///   bar: u32,
///   #[serde(skip)]
///   __json: Option<&'a str>
/// }
/// ```
pub trait Signed<'a>: Deserialize<'a> + Serialize {
    /// Return as a string the signed JSON associated with this struct. Returns None if there no
    /// signed JSON is currently associated with the struct.
    fn json(&self) -> Option<String>;

    /// Remove the JSON string from the struct. This should be called by setter methods that modify
    /// the contents of the struct.
    fn clear_json(&mut self);

    /// Set the JSON string associated with this struct.
    ///
    /// This method should be private. It should only be called from the other methods of this
    /// trait.
    fn set_json(&mut self, _json: &str);

    /// Create a struct of type `T` from the contents of the given JSON string.
    ///
    /// Return the created struct if there is an error.
    fn from_json_string<T>(_json: &str) -> Result<T, anyhow::Error>
    where
        T: Signed<'a>,
    {
        todo!()
    }

    /// If this struct does not have an associated JSON representation then create it and pass it to
    /// the `set_json` method.
    ///
    /// Add a signature to the JSON using the contents of the given key pair.
    fn sign(
        &mut self,
        _signature_algorithm: SignatureAlgorithms,
        _key_pair: &RsaKeyPair,
    ) -> Result((), anyhow::Error) {
        todo!()
    }
}

/// An enumeration of the supported signature algorithms
pub enum SignatureAlgorithms {
    RsaPkcs1Sha512,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Foo<'a> {
        #[serde(skip)]
        __json: Option<&'a str>,
    }

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
