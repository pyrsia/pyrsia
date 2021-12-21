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

//! This module defines structs and traits that are used to implement _signed structs_. A signed
//! struct has associated with it a JSON representation of the struct's contents that are signed.
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
//! use pyrsia_node::signed::{SignatureKeyPair, create_key_pair, JwsSignatureAlgorithms};
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
//! example in the source for `signed_struct_test/lib.rs'. This is the best example of how to use
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

extern crate anyhow;
extern crate base64;
extern crate detached_jws;
extern crate iso8601;
extern crate log;
extern crate openssl;
extern crate serde;
extern crate serde_jcs;
extern crate serde_json;
extern crate time;

pub mod algorithms;
pub mod attestation;

use std::char::REPLACEMENT_CHARACTER;
use std::io::Write;
use std::option::Option;

use crate::signed::json_parser::{parse, JsonPathElement};
use anyhow::{anyhow, Context, Result};
use detached_jws::{DeserializeJwsWriter, SerializeJwsWriter};
use log::{debug, trace, warn};
use openssl::pkey::{PKey, Private};
use openssl::{
    rsa::{Padding, Rsa},
    sign::{Signer, Verifier},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use algorithms::JwsSignatureAlgorithms;
use attestation::Attestation;

// The default size for RSA keys
const DEFAULT_RSA_KEY_SIZE: u32 = 8192;

const ISO8601_FORMAT: &str = "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z";

/// An instance of this struct is created to hold a key pair. The struct can be serialized to save
/// the key pair for later use.
///
/// This struct has the public and private keys as separate values in anticipation that quantum-
/// resistant signature algorithms will require this.
#[derive(Deserialize, Serialize)]
pub struct SignatureKeyPair {
    pub signature_algorithm: JwsSignatureAlgorithms,
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

/// Create and return a key pair using the specified signature algorithm.
pub fn create_key_pair(
    signature_algorithm: JwsSignatureAlgorithms,
) -> Result<SignatureKeyPair, anyhow::Error> {
    match signature_algorithm {
        JwsSignatureAlgorithms::RS512 | JwsSignatureAlgorithms::RS384 => {
            let rsa_private: Rsa<Private> = Rsa::generate(DEFAULT_RSA_KEY_SIZE)?;
            Ok(SignatureKeyPair {
                signature_algorithm,
                private_key: rsa_private.private_key_to_der()?,
                public_key: rsa_private.public_key_to_der()?,
            })
        }
    }
}

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
/// use serde::{Deserialize, Serialize};
/// //noinspection NonAsciiCharacters
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Foo<'a> {
///   foo: &'a str,
///   bar: u32,
///   #[serde(skip)]
///   _json: Option<String>
/// }
/// ```
pub trait Signed<'a>: Deserialize<'a> + Serialize {
    /// Return as a string the signed JSON associated with this struct. Returns None if no
    /// signed JSON is currently associated with the struct.
    fn json(&self) -> Option<String>;

    /// Remove the JSON string from the struct. This should be called by setter methods that modify
    /// the contents of the struct.
    fn clear_json(&mut self);

    /// Set the JSON string associated with this struct.
    ///
    /// This method should be private. It should only be called from the other methods of this
    /// trait. **In the future, an update to the `#[signed_struct]` macro will make the presence of
    /// this method in the train unnecessary and it will be removed from the trait.**
    fn set_json(&mut self, _json: &str);

    /// Create a struct of type `T` from the contents of the given JSON string.
    /// For example, if you have defined a signed struct named `Foo` and you have some JSON created
    /// by calling the `json()` method on an instance of `Foo`, then you can recreate the instance
    /// of `Foo` by writing <br>
    /// `    let my_foo: Foo = Foo::from_json_string(json);
    fn from_json_string<T: Signed<'a>>(json: &'a str) -> serde_json::error::Result<T> {
        let mut signed_struct: T = serde_json::from_str(json)?;
        signed_struct.set_json(json);
        Ok(signed_struct)
    }

    /// If this struct does not have an associated JSON representation then create it and pass it to
    /// the `set_json` method.
    ///
    /// Add a signature to the JSON using the contents of a given key pair.
    /// * signature_algorithm — The signature algorithm to use for signing. Must be compatible with the private key.
    /// * private_key — The der encoded private key to use for signing.
    fn sign_json(
        &mut self,
        signature_algorithm: JwsSignatureAlgorithms,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<(), anyhow::Error> {
        let target_json = string_to_unicode_32(&serde_jcs::to_string(self)?);
        let signed_json = with_signer(
            signature_algorithm,
            private_key,
            public_key,
            &target_json,
            add_signature,
        )?;
        self.set_json(signed_json.as_str());
        Ok(())
    }

    // TODO Add a way to add an expiration time, role and other attributes to signatures.

    /// Verify the signature(s) of this struct's associated JSON.
    ///
    /// Returns an Attestation struct for each signature. These contain information about the
    /// signatures, including whether each one is verified.
    /// If there are no valid signatures an error is returned.
    fn verify_signature(&self) -> Result<Vec<Attestation>, anyhow::Error> {
        self.json().map_or(Err(anyhow!(NOT_SIGNED)), |json| {
            verify_json_signature(&json)
        })
    }

    /// Return the signature information in the signed JSON associated with this struct.
    fn signatures(&self) -> Option<&str> {
        todo!()
    }
}

fn verify_json_signature(json: &str) -> Result<Vec<Attestation>, anyhow::Error> {
    debug!("Verifying signatures: {}", json);
    let mut signature_count = 0;
    let json32 = string_to_unicode_32(json);
    let JsonStringSlices {
        before_signatures,
        signatures,
        after_signatures,
    } = parse_signatures(&json32)?;
    let colon_index = match slice_find(signatures, u32::from(':')) {
        Some(i) => i,
        None => return Err(anyhow!("Corrupt jws")),
    };
    let signatures = &signatures[colon_index + 1..];
    let mut attestations: Vec<Attestation> = Vec::new();
    loop {
        let this_path = vec![JsonPathElement::Index(signature_count)];
        match parse(signatures, &this_path) {
            Ok(json_parser::ParseResult { target, .. }) => {
                let this_signature = target;
                if this_signature.is_empty() {
                    break;
                }
                trace!(
                    "verify_json_signature: this_signature={}",
                    unicode_32_bit_to_string(this_signature)
                );
                attestations.push(verify_one_signature(
                    before_signatures,
                    this_signature,
                    after_signatures,
                ))
            }
            Err(_) => {
                trace!("No more signatures");
                break;
            }
        }
        signature_count += 1;
    }
    trace!("signature_count={}", signature_count);
    if signature_count == 0 {
        return Err(anyhow!(NOT_SIGNED));
    }
    Ok(attestations)
}

// Since the purpose of this function is to find the index of an element in a slice, it really does
// need to have a range loop.
#[allow(clippy::needless_range_loop)]
fn slice_find<T: PartialEq>(slice: &[T], x: T) -> Option<usize> {
    for i in 0..slice.len() {
        if slice[i] == x {
            return Some(i);
        }
    }
    None
}

struct JsonStringSlices<'a> {
    before_signatures: &'a [u32],
    signatures: &'a [u32],
    after_signatures: &'a [u32],
}

fn parse_signatures(json32: &[u32]) -> Result<JsonStringSlices, anyhow::Error> {
    let signature_path: Vec<JsonPathElement> =
        Vec::from([json_parser::JsonPathElement::Field(SIGNATURE_FIELD_NAME)]);
    let json_parser::ParseResult {
        before_target,
        target,
        after_target,
    } = parse(json32, &signature_path)?;
    Ok(JsonStringSlices {
        before_signatures: before_target,
        signatures: target,
        after_signatures: after_target,
    })
}

const EMPTY_ATTESTATION: Attestation = Attestation {
    signature_algorithm: None,
    signature_is_valid: false,
    timestamp: None,
    public_key: None,
    expiration_time: None,
};

// Verify one signature in
fn verify_one_signature(
    before_signatures: &[u32],
    this_jws: &[u32],
    after_signatures: &[u32],
) -> Attestation {
    trace!(
        "verify_one_signature: before=\"{}\"; after=\"{}\"; jws=\"{}\"",
        unicode_32_bit_to_string(before_signatures),
        unicode_32_bit_to_string(after_signatures),
        unicode_32_bit_to_string(this_jws)
    );
    // this_signature is a json string that contains a JWS. We will ignore the enclosing quotes and get the content as a string that is a JWS.
    let jws = unicode_32_bit_to_string(&this_jws[1..this_jws.len() - 1]);
    let jws_header = match header_from_jws(&jws) {
        Some(string) => string,
        None => return EMPTY_ATTESTATION,
    };
    let mut attestation = Attestation::from_json(&jws_header);
    attestation.signature_is_valid = attestation.public_key.as_ref().map_or(false, |public_key| {
        attestation
            .signature_algorithm
            // This is RSA specific. This should be generalized to support other types of signatures.
            .map_or(false, |alg| {
                Rsa::public_key_from_der(public_key).map_or(false, |rsa_key| {
                    PKey::from_rsa(rsa_key).map_or(false, |pkey| {
                        Verifier::new(alg.as_message_digest(), &pkey).map_or(
                            false,
                            |mut verifier| {
                                verifier
                                    .set_rsa_padding(Padding::PKCS1_PSS)
                                    .map_or(false, |_| {
                                        DeserializeJwsWriter::new(&jws, |_| Some(verifier)).map_or(
                                            false,
                                            |mut writer| {
                                                writer
                                                    .write(
                                                        unicode_32_bit_to_string(before_signatures)
                                                            .as_bytes(),
                                                    )
                                                    .map_or(false, |_| {
                                                        writer
                                                            .write(
                                                                unicode_32_bit_to_string(
                                                                    after_signatures,
                                                                )
                                                                .as_bytes(),
                                                            )
                                                            .map_or(false, |_| {
                                                                writer
                                                                    .finish()
                                                                    .map_or(false, |_| true)
                                                            })
                                                    })
                                            },
                                        )
                                    })
                            },
                        )
                    })
                })
            })
    });
    attestation
}

fn header_from_jws(jws: &str) -> Option<String> {
    let first_dot_index = match jws.find('.') {
        Some(i) => i,
        None => return None,
    };
    // if decode fails treat it as a missing signature. Because this is security, we don't want to be helpful if the JWS is not correctly formatted.
    match base64::decode_config(&jws[..first_dot_index], base64::STANDARD_NO_PAD) {
        Ok(decoded_json) => match String::from_utf8(decoded_json) {
            Ok(string) => Some(string),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// preprocess a json string to a Vec<32> whose elements each contain exactly one unicode character.
fn string_to_unicode_32(raw: &str) -> Vec<u32> {
    let mut v = Vec::with_capacity(raw.len());
    raw.chars().for_each(|c| {
        v.push(u32::from(c));
    });
    v
}

const SIGNATURE_FIELD_NAME: &str = "__signature";
const SIGNER_FIELD_NAME: &str = "signer";
const ALG_FIELD_NAME: &str = "alg";
const TIMESTAMP_FIELD_NAME: &str = "timestamp";
const EXPIRATION_FIELD_NAME: &str = "ext";

// Error Strings
const NOT_SIGNED: &str = "Not signed!";

type SigningFunction =
    fn(JwsSignatureAlgorithms, Signer, &[u8], &[u32]) -> Result<String, anyhow::Error>;

// construct a signer, pass it to the given signing function and then return the signed json returned from the signing function.
fn with_signer(
    signature_algorithm: JwsSignatureAlgorithms,
    der_private_key: &[u8],
    der_public_key: &[u8],
    target_json: &[u32],
    signing_function: SigningFunction,
) -> Result<String, anyhow::Error> {
    // This is RSA specific. This should be generalized to support other types of signatures.
    let private_key: Rsa<Private> = Rsa::private_key_from_der(der_private_key)?;
    let kp: PKey<Private> = PKey::from_rsa(private_key)?;
    let mut signer = Signer::new(signature_algorithm.as_message_digest(), &kp)
        .context("Problem using key pair")?;
    signer.set_rsa_padding(Padding::PKCS1_PSS)?;
    signing_function(signature_algorithm, signer, der_public_key, target_json)
}

fn add_signature(
    signature_algorithm: JwsSignatureAlgorithms,
    signer: Signer,
    der_public_key: &[u8],
    target_json: &[u32],
) -> Result<String, anyhow::Error> {
    let json_parser::ParseResult {
        before_target,
        target,
        after_target,
    } = json_parser::parse(
        target_json,
        &[json_parser::JsonPathElement::Field(SIGNATURE_FIELD_NAME)],
    )?;
    let header = create_jsw_header(der_public_key);
    let mut before_string = unicode_32_bit_to_string(before_target);
    before_string.push(',');
    let after_string = unicode_32_bit_to_string(after_target);
    let jws = create_jws(
        signature_algorithm,
        signer,
        &before_string,
        &after_string,
        header,
    )?;
    let jws_string = String::from_utf8(jws)?;
    let mut signed_json_buffer = before_string.clone();
    if target.is_empty() {
        // No existing signatures
        signed_json_buffer.push('"');
        signed_json_buffer.push_str(SIGNATURE_FIELD_NAME);
        signed_json_buffer.push_str(r#"":[""#);
    } else {
        signed_json_buffer.push_str(unicode_32_bit_to_string(&target[..target.len() - 1]).as_str()); // append signature array without closing ']'
        signed_json_buffer.push_str(",\"");
    }
    signed_json_buffer.push_str(jws_string.as_str());
    signed_json_buffer.push_str("\"]");
    signed_json_buffer.push_str(&after_string);
    Ok(signed_json_buffer)
}

fn create_jws(
    signature_algorithm: JwsSignatureAlgorithms,
    signer: Signer,
    before: &str,
    after: &str,
    header: Map<String, Value>,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut writer = SerializeJwsWriter::new(
        Vec::new(),
        signature_algorithm.to_jws_name(),
        header,
        signer,
    )?;
    writer.write_all(before.as_bytes())?;
    writer.write_all(after.as_bytes())?;
    let jws = writer.finish()?;
    Ok(jws)
}

fn unicode_32_bit_to_string(u: &[u32]) -> String {
    let mut s = String::with_capacity(u.len() * 4);
    u.iter()
        .for_each(|u32| s.push(char::from_u32(*u32).unwrap_or(REPLACEMENT_CHARACTER)));
    s
}

fn iso8601_format_spec() -> Vec<FormatItem<'static>> {
    format_description::parse(ISO8601_FORMAT).unwrap() // Call unwrap because this format spec is tested and should never fail. If it does, there is nothing to do but panic.
}

fn now_as_iso8601_string() -> String {
    OffsetDateTime::now_utc()
        .format(&iso8601_format_spec())
        .unwrap() // If the formatting fails there is no reasonable action but panic.
}

fn create_jsw_header(public_key: &[u8]) -> Map<String, Value> {
    let mut header = Map::new();
    header.insert(
        SIGNER_FIELD_NAME.to_owned(),
        json!(base64::encode_config(public_key, base64::STANDARD_NO_PAD)),
    );
    let now_string = now_as_iso8601_string();
    trace!("Timestamping signature at {}", now_string);
    header.insert(
        TIMESTAMP_FIELD_NAME.to_owned(),
        json!(format!("{:?}", now_string)),
    );
    header
}

/// Lightweight JSON parser to identify the portion of a slice before and after a value, so that the
/// value can easily be replaced.
mod json_parser {
    use super::string_to_unicode_32;
    use crate::signed::unicode_32_bit_to_string;
    use anyhow::anyhow;
    use log::{debug, trace};
    use std::char::REPLACEMENT_CHARACTER;
    use std::slice::Iter;

    pub struct JsonCursor<'a> {
        position: usize,
        iterator: Iter<'a, u32>,
        this_char: Option<&'a u32>,
        json_str: &'a [u32],
    }

    impl<'a> JsonCursor<'a> {
        pub fn new(json: &[u32]) -> JsonCursor {
            let mut iterator = json.iter();
            let this_char = iterator.next();
            JsonCursor {
                position: 0,
                iterator,
                this_char,
                json_str: json,
            }
        }

        fn next(&mut self) {
            if self.this_char.is_some() {
                self.position += 1;
            }
            self.this_char = self.iterator.next();
        }

        fn this_char_equals(&self, c: u32) -> bool {
            self.this_char.is_some() && *self.this_char.unwrap() == c
        }

        fn expect_char(&mut self, next_char: u32) -> Result<(), anyhow::Error> {
            if self.this_char.is_some() && *self.this_char.unwrap() == next_char {
                self.next();
                Ok(())
            } else {
                let mut found_char = String::new();
                if self.this_char.is_some() {
                    found_char.push(
                        char::from_u32(*self.this_char.unwrap()).unwrap_or(REPLACEMENT_CHARACTER),
                    )
                } else {
                    found_char.push_str("None")
                }
                Err(anyhow!(format!(
                    "Expected '{}' but found '{}' at position {}.",
                    next_char, found_char, self.position
                )))
            }
        }

        fn char_predicate(&self, predicate: fn(u32) -> bool) -> bool {
            self.this_char.map_or(false, |c| predicate(*c))
        }

        fn at_end(&self) -> bool {
            self.this_char.is_none()
        }

        fn skip_char(&mut self, c: u32) {
            skip_whitespace(self);
            if self.this_char_equals(c) {
                self.next();
            }
        }
    }

    #[derive(Clone)]
    pub enum JsonPathElement<'a> {
        Field(&'a str),
        Index(usize),
    }

    pub struct ParseResult<'a> {
        pub before_target: &'a [u32],
        pub target: &'a [u32],
        pub after_target: &'a [u32],
    }

    /// Given a string slice that contains JSON and the path of a value, this returns three smaller
    /// slices that are the characters before a specified value, the characters that comprise the value
    /// and the characters after the value.
    ///
    /// If the path has more than one element and an element of the path other than the last is not
    /// found, that is treated as an error.
    ///
    /// If the last element of the path is not found, then the result have an empty middle slice,
    /// positioned where such an element could be inserted.
    pub fn parse<'a>(
        json: &'a [u32],
        path: &[JsonPathElement],
    ) -> Result<ParseResult<'a>, anyhow::Error> {
        if path.is_empty() {
            debug!("parse: Empty path; nothing to find");
            return Err(anyhow!("Empty path; nothing to find"));
        }
        trace!("parse: parsing {}", unicode_32_bit_to_string(json));
        let mut start_of_target: usize = 0;
        let mut end_of_target: usize = 0;
        parse_value(
            &mut start_of_target,
            &mut end_of_target,
            &mut path.iter(),
            &mut JsonCursor::new(json),
        )?;
        if end_of_target == 0 && end_of_target <= start_of_target || end_of_target < start_of_target
        {
            debug!("parse: Did not find {}", path_to_str(path));
            return Err(anyhow!(format!("Did not find {}", path_to_str(path))));
        }
        Ok(ParseResult {
            before_target: &json[..(start_of_target)],
            target: &json[start_of_target..end_of_target],
            after_target: &json[end_of_target..],
        })
    }

    fn parse_value(
        start_of_target: &mut usize,
        end_of_target: &mut usize,
        path: &mut core::slice::Iter<JsonPathElement>,
        json_cursor: &mut JsonCursor,
    ) -> Result<(), anyhow::Error> {
        match path.next() {
            Some(JsonPathElement::Field(field_name)) => parse_object(
                start_of_target,
                end_of_target,
                path,
                json_cursor,
                Some(&string_to_unicode_32(field_name)),
            ),
            Some(JsonPathElement::Index(index)) => parse_array(
                start_of_target,
                end_of_target,
                path,
                json_cursor,
                Some(*index),
            ),
            None => parse_unconstrained_value(start_of_target, end_of_target, json_cursor),
        }
    }

    fn parse_unconstrained_value(
        start_of_target: &mut usize,
        end_of_target: &mut usize,
        json_cursor: &mut JsonCursor,
    ) -> Result<(), anyhow::Error> {
        skip_whitespace(json_cursor);
        if json_cursor.this_char_equals(u32::from('{')) {
            parse_object(
                start_of_target,
                end_of_target,
                &mut Vec::new().iter(),
                json_cursor,
                None,
            )
        } else if json_cursor.this_char_equals(u32::from('[')) {
            parse_array(
                start_of_target,
                end_of_target,
                &mut Vec::new().iter(),
                json_cursor,
                None,
            )
        } else if json_cursor.this_char_equals(u32::from('"')) {
            parse_string(json_cursor)?;
            Ok(())
        } else if json_cursor.char_predicate(is_signed_alphanumeric) {
            parse_number_or_id(json_cursor)
        } else {
            Err(anyhow!(format!(
                "Unexpected character '{}' at position {} in json: {}",
                &json_cursor
                    .this_char
                    .map_or(String::from("None"), |c| String::from(
                        char::from_u32(*c).unwrap_or(REPLACEMENT_CHARACTER)
                    )),
                json_cursor.position,
                unicode_32_bit_to_string(json_cursor.json_str)
            )))
        }
    }

    fn is_signed_alphanumeric(u: u32) -> bool {
        char::from_u32(u).map_or(false, |c| c.is_alphanumeric() || c == '-' || c == '+')
    }

    // Parse a number or an word like "null", "true" or "false". Since we are just scanning to find
    // the end of something, we don't need to care about the distinctions.
    fn parse_number_or_id(json_cursor: &mut JsonCursor) -> Result<(), anyhow::Error> {
        while json_cursor.char_predicate(is_signed_alphanumeric) {
            json_cursor.next()
        }
        Ok(())
    }

    fn parse_array(
        start_of_target: &mut usize,
        end_of_target: &mut usize,
        path: &mut core::slice::Iter<JsonPathElement>,
        json_cursor: &mut JsonCursor,
        target_index: Option<usize>,
    ) -> Result<(), anyhow::Error> {
        skip_whitespace(json_cursor);
        json_cursor.expect_char(u32::from('['))?;
        let mut this_index: usize = 0;
        let is_empty_path = path.clone().next().is_none();
        loop {
            let start_position = json_cursor.position;
            skip_whitespace(json_cursor);
            if json_cursor.at_end() {
                return Err(anyhow!(format!(
                    "Unterminated array started at position {}",
                    start_position
                )));
            }
            if json_cursor.this_char_equals(u32::from(']')) {
                if target_index.is_some() && is_empty_path {
                    // path target not found. Pretend we found it at the end of the array as an empty string
                    *start_of_target = start_position;
                    *end_of_target = json_cursor.position;
                }
                json_cursor.next();
                return Ok(());
            }
            if target_index.unwrap_or(usize::MAX) == this_index {
                parse_value(start_of_target, end_of_target, path, json_cursor)?;
                json_cursor.skip_char(u32::from(','));
                if is_empty_path {
                    // This is the JSON array index identified by the path
                    *start_of_target = start_position;
                    *end_of_target = json_cursor.position;
                }
                return Ok(());
            } else {
                parse_value(
                    start_of_target,
                    end_of_target,
                    &mut Vec::new().iter(),
                    json_cursor,
                )?;
                json_cursor.skip_char(u32::from(','));
            }
            this_index += 1
        }
    }

    fn parse_object(
        start_of_target: &mut usize,
        end_of_target: &mut usize,
        path: &mut core::slice::Iter<JsonPathElement>,
        json_cursor: &mut JsonCursor,
        target_field: Option<&[u32]>,
    ) -> Result<(), anyhow::Error> {
        let is_empty_path = path.clone().next().is_none();
        skip_whitespace(json_cursor);
        json_cursor.expect_char(u32::from('{'))?;
        loop {
            let start_position = json_cursor.position;
            skip_whitespace(json_cursor);
            if json_cursor.at_end() {
                return Err(anyhow!(format!(
                    "Unterminated object started at position {}",
                    start_position
                )));
            }
            if json_cursor.this_char_equals(u32::from('}')) {
                if target_field.is_some() && is_empty_path {
                    // path target not found. Pretend we found it at the end of the object as an empty string
                    *start_of_target = start_position;
                    *end_of_target = json_cursor.position;
                }
                json_cursor.next();
                return Ok(());
            };
            let field_name = parse_string(json_cursor)?;
            skip_whitespace(json_cursor);
            json_cursor.expect_char(u32::from(':'))?;
            if target_field.unwrap_or_default() == field_name {
                parse_value(start_of_target, end_of_target, path, json_cursor)?;
                if is_empty_path {
                    json_cursor.skip_char(u32::from(','));
                    // This is the JSON field identified by the path
                    *start_of_target = start_position;
                    *end_of_target = json_cursor.position;
                };
                return Ok(());
            } else {
                // not part of the path, so we parse just to scan past it.
                parse_value(
                    start_of_target,
                    end_of_target,
                    &mut Vec::new().iter(),
                    json_cursor,
                )?;
                json_cursor.skip_char(u32::from(','))
            }
        }
    }

    pub fn parse_string(json_cursor: &mut JsonCursor) -> Result<Vec<u32>, anyhow::Error> {
        skip_whitespace(json_cursor);
        json_cursor.expect_char(u32::from('"'))?;
        let string_start = json_cursor.position;
        loop {
            if json_cursor.at_end() {
                return Err(anyhow!(format!(
                    "JSON contains an unterminated string that starts at position {}.",
                    string_start
                )));
            }
            if json_cursor.this_char_equals(u32::from('\\')) {
                json_cursor.next(); // Ignore the next character because it is escaped.
            } else if json_cursor.this_char_equals(u32::from('"')) {
                let content = &json_cursor.json_str[string_start..json_cursor.position];
                json_cursor.next();
                return Ok(Vec::from(content));
            }
            json_cursor.next();
        }
    }

    fn skip_whitespace(json_cursor: &mut JsonCursor) {
        while json_cursor.char_predicate(is_whitespace) {
            json_cursor.next()
        }
    }

    fn is_whitespace(u: u32) -> bool {
        u == 0x09
            || u == 0x0a
            || u == 0x0d
            || u == 0x20
            || u == 0x00a0
            || u == 0x1680
            || u == 0x180e
            || (0x2000..=0x200b).contains(&u)
            || u == 0x202f
            || u == 0x205f
            || u == 0x3000
            || u == 0xfeff
    }

    pub fn path_to_str(path: &[JsonPathElement]) -> String {
        let mut s = String::from("path[");
        if !path.is_empty() {
            path_element_to_str(&mut s, &path[0]);
            for path_element in path[1..].iter() {
                s.push_str("\",");
                path_element_to_str(&mut s, path_element);
            }
        }
        s.push(']');
        s
    }

    fn path_element_to_str(s: &mut String, path_element: &JsonPathElement) {
        match path_element {
            JsonPathElement::Field(field_name) => {
                s.push_str("field:\"");
                s.push_str(field_name);
            }
            JsonPathElement::Index(index) => {
                s.push_str(index.to_string().as_str());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use json_parser::*;
    use log::info;

    //noinspection NonAsciiCharacters
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        foo: &'a str,
        bar: u32,
        zot: &'a str,
        #[serde(skip)]
        _json: Option<String>,
    }

    impl<'a> Signed<'a> for Foo<'a> {
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

    #[test]
    fn path_to_string_test() {
        env_logger::try_init().unwrap_or_default();
        let path = vec![
            JsonPathElement::Field("__signature"),
            JsonPathElement::Index(4),
        ];
        assert_eq!("path[field:\"__signature\",4]", path_to_str(&path))
    }

    #[test]
    // The purpose of the parse function that this tests is to find a specified object field or
    // array element in a given JSON string. The parse function returns three slices of the original
    // string: the target that is was looking for, the portion of the string before the target and
    // the portion of the string after the target.
    //
    // This test specifies a piece of JSON that includes all of the cases that need to be tested and
    // then calls the parse method to look for different things in the string. For each search, it
    // checks that the three slices are correct.
    fn parse_json() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();
        let json = r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#;
        let json32 = string_to_unicode_32(json);
        let test = |expected_before: &str,
                    expected_middle: &str,
                    expected_after: &str,
                    path: Vec<JsonPathElement>|
         -> Result<(), anyhow::Error> {
            let ParseResult {
                before_target,
                target,
                after_target,
            } = parse(&json32, &path)?;

            assert_eq!(expected_before, unicode_32_bit_to_string(before_target));
            assert_eq!(expected_middle, unicode_32_bit_to_string(target));
            assert_eq!(expected_after, unicode_32_bit_to_string(after_target));
            Ok(())
        };
        test(
            "{",
            r#""boo":true,"#,
            r#""number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("boo")],
        )?;
        test(
            r#"{"boo":true,"#,
            r#""number":234,"#,
            r#""nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("number")],
        )?;
        test(
            r#"{"boo":true,"number":234,"#,
            r#""nul":null,"#,
            r#" "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("nul")],
        )?;
        test(
            r#"{"boo":true,"number":234,"nul":null,"#,
            r#" "ob":{"a":123,"b":"str"},"#,
            r#" "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("ob")],
        )?;
        test(
            r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"},"#,
            r#" "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] ,"#,
            r#" "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("arr")],
        )?;
        test(
            r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true,"#,
            r#" {"sig":"mund", "om":"ega"},"#,
            r#" "asfd"] , "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("arr"), JsonPathElement::Index(2)],
        )?;
        test(
            r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"#,
            r#""sig":"mund","#,
            r#" "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![
                JsonPathElement::Field("arr"),
                JsonPathElement::Index(2),
                JsonPathElement::Field("sig"),
            ],
        )?;
        test(
            r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega""#,
            r#""#,
            r#"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![
                JsonPathElement::Field("arr"),
                JsonPathElement::Index(2),
                JsonPathElement::Field("Zog"),
            ],
        )?;
        Ok(())
    }

    #[test]
    fn parse_string_happy_test() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();
        let test32 = string_to_unicode_32("  \"The quick Brown fox.\" ");
        let mut cursor = JsonCursor::new(&test32);
        let parsed_string = parse_string(&mut cursor)?;
        let expected = string_to_unicode_32("The quick Brown fox.");
        assert_eq!(expected, parsed_string);
        Ok(())
    }

    #[test]
    fn parse_string_escape() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();
        let test32 = string_to_unicode_32("  \"The quick \\\"Brown\\\" fox.\" ");
        let mut cursor = JsonCursor::new(&test32);
        let parsed_string = parse_string(&mut cursor)?;
        let expected = string_to_unicode_32(r#"The quick \"Brown\" fox."#);
        assert_eq!(expected, parsed_string);
        Ok(())
    }

    #[test]
    fn parse_string_unterminated() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();
        let test32 = string_to_unicode_32("  \"The quick \\\"Brown\\\" fox. ");
        let mut cursor = JsonCursor::new(&test32);
        match parse_string(&mut cursor) {
            Ok(_) => Err(anyhow!(
                "Parsing an unterminated string did not cause an error return!"
            )),
            Err(_) => Ok(()),
        }
    }

    #[test]
    fn parse_iso8601_test() {
        let now = now_as_iso8601_string();
        let odt = parse_iso8601(&now);
        assert!(odt.is_some());
        let dt = odt.unwrap();
        assert_eq!(2021, dt.year());
    }

    #[test]
    fn happy_path_for_signing() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();
        // create a key pair for other signing types to see that they succeed
        let key_pair = super::create_key_pair(JwsSignatureAlgorithms::RS512)?;
        info!("Created key pair");

        let mut foo = Foo {
            foo: "π is 16 bit unicode",
            bar: 23894,
            zot: "🦽is 32 bit unicode",
            _json: None,
        };
        assert!(foo.json().is_none());
        foo.sign_json(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )
        .context("Error signing struct")?;
        info!("Signed json from foo {}", foo.json().unwrap());
        let attestations = foo.verify_signature()?;
        assert_eq!(1, attestations.len());
        assert!(attestations[0].signature_is_valid);
        let json = foo.json();
        assert!(json.is_some());
        let json_string = json.unwrap();
        let foo2: Foo = Foo::from_json_string(&json_string).unwrap();
        assert_eq!(foo, foo2);
        foo2.verify_signature()?;
        Ok(())
    }
}
