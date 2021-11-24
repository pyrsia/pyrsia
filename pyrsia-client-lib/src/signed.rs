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
/// If we modify the contents of a signed struct, we should discard the JSON since it no longer
/// matches the struct. When the struct is in an unsigned state, it should not be allowed to
/// serialize it.
///
/// Given the above description of the purposes of the `Signed` trait, the descriptions of its
/// methods should be understood in this context.
///
/// It is recommended for consistency that structs that implement this trait have a field declared
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
    /// Return as a string the JSON associated with this struct.
    fn json(&self) -> Option<String>;

    /// Remove the JSON string from the struct. This should be called by setter methods that modify
    /// the contents of the struct.
    fn clear_json(&mut self) ;

    fn from_json_string<T>(json: &str) -> Result<T, anyhow::Error> where T: Signed<'a> {
        todo!()
    }

    fn sign(&mut self, algorithm: SignatureAlgorithms, key_pair: &RsaKeyPair) {
        todo!()
    }

}

/// An enumeration of the supported signature algorithms
pub enum SignatureAlgorithms {
    RsaPkcs1Sha512
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
