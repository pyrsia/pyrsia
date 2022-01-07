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
//! This is our way of signing data and verifying signatures.
//!
//! This should not be used directly, but using the<br>
//! #[signed]
//! macro. This is documented in `src/signed_struct/tests/lib.rs

extern crate anyhow;
extern crate base64;
extern crate chrono;
extern crate log;
extern crate openssl;
extern crate serde;
extern crate serde_json;

use std::char::REPLACEMENT_CHARACTER;
use std::option::Option;

use crate::signed::json_parser::{parse, JsonPathElement};
use anyhow::{anyhow, Context, Result};
use base64::decode_config;
use base64::write::EncoderWriter;
use chrono::{DateTime, SecondsFormat, Utc};
use log::Level::Trace;
use log::{debug, log_enabled, trace, warn};
use openssl::pkey::{PKey, Private};
use openssl::{
    hash::MessageDigest,
    rsa::{Padding, Rsa},
    sign::{Signer, Verifier},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// An enumeration of the supported signature algorithms used by this module to sign structs and
/// JSON.
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq)]
pub enum JwsSignatureAlgorithms {
    RS512,
    RS384,
}

impl JwsSignatureAlgorithms {
    /// Return a string that contains the JWS name of this.
    pub fn to_jws_name(&self) -> String {
        String::from(match self {
            JwsSignatureAlgorithms::RS512 => "RS512",
            JwsSignatureAlgorithms::RS384 => "RS384",
        })
    }

    /// Return a MessageDigest appropriate for the algorithm.
    fn as_message_digest(&self) -> MessageDigest {
        match self {
            JwsSignatureAlgorithms::RS512 => MessageDigest::sha512(),
            JwsSignatureAlgorithms::RS384 => MessageDigest::sha384(),
        }
    }

    /// Return the supported algorithm that corresponds to the given JWS name.
    pub fn from_jws_name(jws_name: &str) -> Option<JwsSignatureAlgorithms> {
        let name = jws_name.to_uppercase();
        if "RS512" == name {
            Some(JwsSignatureAlgorithms::RS512)
        } else if "RS384" == name {
            Some(JwsSignatureAlgorithms::RS384)
        } else {
            None
        }
    }
}

// The default size for RSA keys
const DEFAULT_RSA_KEY_SIZE: u32 = 8192;

/// This contains the information from an individual verified signature of a struct or JSON.
/// When the signature(s) of a signed struct are verified, one of this structs is produced to
/// provide information about each of the signatures.
///
/// That fact that it is validated means that it was signed by the identity associated with the
/// public key and that the contents have not been modified since they were signed. This information
/// is provided so that you can reason about the trust-worthiness of the signature.
pub struct Attestation {
    public_key: Option<Vec<u8>>,
    signature_algorithm: Option<JwsSignatureAlgorithms>,
    timestamp: Option<DateTime<Utc>>,
    expiration_time: Option<DateTime<Utc>>,
    signature_is_valid: bool,
}

impl Attestation {
    /// The public key of the signer
    pub fn public_key(&self) -> &Option<Vec<u8>> {
        &self.public_key
    }

    /// The signature algorithm
    pub fn signature_algorithm(&self) -> &Option<JwsSignatureAlgorithms> {
        &self.signature_algorithm
    }

    /// The timestamp of the signature
    pub fn timestamp(&self) -> &Option<DateTime<Utc>> {
        &self.timestamp
    }

    /// The optional expiration time of the signature
    pub fn expiration_time(&self) -> &Option<DateTime<Utc>> {
        &self.expiration_time
    }

    /// True if signature verification determined that the signature is valid.
    pub fn signature_is_valid(&self) -> bool {
        self.signature_is_valid
    }

    // create an attestation with all the information from the JWS header.
    // The is_valid field is set to false. It is the responsibility of the caller to change it if valid.
    fn from_json(jws_header: &str) -> Attestation {
        let mut attestation = Attestation {
            signature_is_valid: false,
            signature_algorithm: None,
            expiration_time: None,
            timestamp: None,
            public_key: None,
        };
        let json_header: Value = match serde_json::from_str(jws_header) {
            Ok(json) => json,
            Err(_) => return attestation,
        };
        attestation.signature_algorithm = match &json_header[ALG_FIELD_NAME] {
            Value::String(alg_string) => JwsSignatureAlgorithms::from_jws_name(alg_string),
            _ => None,
        };
        attestation.expiration_time = date_time_from_json(&json_header, EXPIRATION_FIELD_NAME);
        attestation.timestamp = date_time_from_json(&json_header, TIMESTAMP_FIELD_NAME);
        attestation.public_key = public_key_from_json(&json_header);
        attestation
    }
}

fn public_key_from_json(json_header: &Value) -> Option<Vec<u8>> {
    match &json_header[SIGNER_FIELD_NAME] {
        Value::String(key_string) => {
            match base64::decode_config(key_string, base64::STANDARD_NO_PAD) {
                Ok(key) => Some(key),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

fn date_time_from_json(json_header: &Value, field_name: &str) -> Option<DateTime<Utc>> {
    match &json_header[field_name] {
        Value::String(time_string) => {
            let unquoted_time_string: &str = time_string[1..time_string.len() - 1].as_ref();
            match DateTime::parse_from_rfc3339(unquoted_time_string) {
                Ok(datetime) => Some(DateTime::from(datetime)),
                Err(err) => {
                    warn!("Datetime value in JSON field {} could not be parsed \"{}\". {}. Treating the field as missing", field_name, err, time_string);
                    None
                }
            }
        }
        _ => None,
    }
}

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
        let existing_json = &self.json();
        let new_json: String;
        let starting_json: &String;
        if existing_json.is_some() {
            starting_json = unwrap(existing_json);
        } else {
            new_json = serde_json::to_string(self)?;
            starting_json = &new_json;
        };
        debug!("Adding signature to json: {}", starting_json);
        let target_json = string_to_unicode_32(starting_json);
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

fn unwrap<T>(o: &Option<T>) -> &T {
    match o {
        Some(t) => t,
        None => panic!("unwrapped None"),
    }
}

fn verify_json_signature(json: &str) -> Result<Vec<Attestation>, anyhow::Error> {
    debug!("Verifying signatures: {}", json);
    let mut signature_count = 0;
    let mut valid_signature_count = 0;
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
                let this_attestation =
                    verify_one_signature(before_signatures, this_signature, after_signatures)
                        .unwrap_or(EMPTY_ATTESTATION);
                if this_attestation.signature_is_valid {
                    valid_signature_count += 1;
                }
                attestations.push(this_attestation);
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
        Err(anyhow!(NOT_SIGNED))
    } else if valid_signature_count == 0 {
        Err(anyhow!("No valid signatures"))
    } else {
        Ok(attestations)
    }
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
) -> Result<Attestation> {
    let before_signatures_string = unicode_32_bit_to_string(before_signatures);
    let after_signatures_string = unicode_32_bit_to_string(after_signatures);
    debug!(
        "verify_one_signature: before={}\n; after={}\n; jws={}",
        &before_signatures_string,
        &after_signatures_string,
        unicode_32_bit_to_string(this_jws)
    );
    let before_signatures_bytes = before_signatures_string.as_bytes();
    let after_signatures_bytes = after_signatures_string.as_bytes();
    // this_signature is a json string that contains a JWS. We will ignore the enclosing quotes and get the content as a string that is a JWS.
    let jws = unicode_32_bit_to_string(&this_jws[1..this_jws.len() - 1]);
    let (encoded_jws_header, encoded_signature) = match split_jws(&jws) {
        Some((encoded_jws_header, encoded_signature)) => (encoded_jws_header, encoded_signature),
        None => return Ok(EMPTY_ATTESTATION),
    };
    let decoded_signature = base64::decode_config(encoded_signature, base64::STANDARD_NO_PAD)?;
    let decoded_jws_header = base64::decode_config(&encoded_jws_header, base64::STANDARD_NO_PAD)?;
    let decoded_jws_header_string = String::from_utf8(decoded_jws_header.clone())?;
    debug!(
        "validating JWS with header: \"{}\"",
        decoded_jws_header_string
    );
    let mut attestation = Attestation::from_json(&decoded_jws_header_string);
    attestation.signature_is_valid = attestation.public_key.as_ref().map_or(false, |public_key| {
        attestation
            .signature_algorithm
            // This is RSA specific. This should be generalized to support other types of signatures.
            .map_or(false, |alg| {
                let rsa_key = match Rsa::public_key_from_der(public_key) {
                    Ok(rsa_key) => rsa_key,
                    Err(_) => return false,
                };
                let pkey = match PKey::from_rsa(rsa_key) {
                    Ok(pkey) => pkey,
                    Err(_) => return false,
                };
                let mut verifier = match Verifier::new(alg.as_message_digest(), &pkey) {
                    Ok(verifier) => verifier,
                    Err(_) => return false,
                };
                if verifier.set_rsa_padding(Padding::PKCS1_PSS).is_err() {
                    return false;
                }
                if log_enabled!(Trace) {
                    trace!("Verification header: {}", encoded_jws_header);
                    trace_u8_slice_output("jws header", &decoded_jws_header[..]);
                }
                if verifier.update(encoded_jws_header.as_bytes()).is_err() {
                    return false;
                }
                trace_u8_slice_output("Verification before", before_signatures_bytes);
                if verifier.update(before_signatures_bytes).is_err() {
                    return false;
                }
                trace_u8_slice_output("after", after_signatures_bytes);
                if verifier.update(after_signatures_bytes).is_err() {
                    return false;
                }
                verifier.verify(&decoded_signature).is_ok()
            })
    });
    Ok(attestation)
}

fn trace_u8_slice_output(label: &str, slice: &[u8]) {
    if log_enabled!(Trace) {
        trace!("{} size={}", label, slice.len());
        for (i, byte) in slice.iter().enumerate() {
            trace!("[{}] = {}", i, byte);
        }
    }
}

fn split_jws(jws: &str) -> Option<(String, String)> {
    let first_dot_index = match jws.find('.') {
        Some(i) => i,
        None => return None,
    };
    let encoded_header = &jws[..first_dot_index];
    let encoded_signature = &jws[first_dot_index + 2..];
    Some((encoded_header.to_string(), encoded_signature.to_string()))
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
    let header = create_jsw_header(der_public_key, signature_algorithm)?;
    let before_string = unicode_32_bit_to_string(before_target);
    let after_string = unicode_32_bit_to_string(after_target);
    let jws = create_jws(
        signature_algorithm,
        signer,
        &before_string,
        &after_string,
        header,
    )?;
    let jws_string = String::from_utf8(jws)?;
    let mut signed_json_buffer = before_string;
    if target.is_empty() {
        // No existing signatures
        signed_json_buffer.push_str(r#",""#);
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
    mut signer: Signer,
    before: &str,
    after: &str,
    header: Map<String, Value>,
) -> Result<Vec<u8>, anyhow::Error> {
    debug!(
        "Signing: SignatureAlgorithm={:?}\nbefore={}\nafter={}\nheader{:?}",
        signature_algorithm, before, after, header
    );
    let encoded_header = encode_header(header)?;
    if log_enabled!(Trace) {
        trace!(
            "Signing header: {}",
            String::from_utf8(encoded_header.clone())?
        );
        trace!(
            "Decoded header: {}",
            String::from_utf8(decode_config(
                encoded_header.clone(),
                base64::STANDARD_NO_PAD
            )?)?
        );
        trace_u8_slice_output("Signing header", &encoded_header);
    }
    signer.update(&encoded_header)?;

    trace_u8_slice_output("signing before", before.as_bytes());
    signer.update(before.as_bytes())?;
    trace_u8_slice_output("signing after", after.as_bytes());
    signer.update(after.as_bytes())?;
    let signature = signer.sign_to_vec()?;
    let mut jws = vec![];
    jws.extend(encoded_header);
    jws.push(b'.');
    jws.push(b'.');
    jws.extend(base64::encode_config(signature, base64::STANDARD_NO_PAD).as_bytes());
    debug!("Encoded jws:{}", &String::from_utf8(jws.clone())?);
    Ok(jws)
}

fn encode_header(header: Map<String, Value>) -> Result<Vec<u8>> {
    let mut encoder = EncoderWriter::new(Vec::new(), base64::STANDARD_NO_PAD);
    serde_json::to_writer(&mut encoder, &header)?;
    Ok(encoder.finish()?)
}

fn unicode_32_bit_to_string(u: &[u32]) -> String {
    let mut s = String::with_capacity(u.len() * 4);
    u.iter()
        .for_each(|u32| s.push(char::from_u32(*u32).unwrap_or(REPLACEMENT_CHARACTER)));
    s
}

// Now with millisecond precision and time zone "Z"
pub fn now_as_iso8601_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn create_jsw_header(
    public_key: &[u8],
    signature_algorithm: JwsSignatureAlgorithms,
) -> Result<Map<String, Value>> {
    let mut header = Map::new();
    let jws_algorithm = signature_algorithm.to_jws_name();

    header.insert(ALG_FIELD_NAME.to_owned(), json!(jws_algorithm));
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
    Ok(header)
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

    // This struct is use by the tests
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Namespace {
        id: String,
        namespace_path: Vec<String>,
        revision_count: u32,
        description: Option<String>,
        creation_time: String,
        // This contains the JSON associated with the struct. It must not be serialized with the rest of the struct.
        #[serde(skip)]
        _json0: Option<String>,
    }

    impl Signed<'_> for Namespace {
        fn json(&self) -> Option<String> {
            self._json0.to_owned()
        }

        fn clear_json(&mut self) {
            self._json0 = None;
        }

        fn set_json(&mut self, json: &str) {
            self._json0 = Option::Some(json.to_string())
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
    // array element in a given JSON string. This is called the target.
    //
    // The parse function returns three slices of the original string: the target that is was
    // looking for, the portion of the string before the target and the portion of the string after
    // the target.
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

    // Test the signing use case
    #[test]
    fn happy_path_for_signing() -> Result<(), anyhow::Error> {
        env_logger::try_init().unwrap_or_default();
        // create a key pair for other signing types to see that they succeed
        let key_pair = super::create_key_pair(JwsSignatureAlgorithms::RS512)?;
        info!("Created key pair");

        // Create the struct instance to be signed
        let mut namespace = Namespace {
            id: "61c23c81-5cee-4d93-83fd-10fd60936fdc".to_string(),
            namespace_path: vec!["docker".to_string()],
            revision_count: 5,
            description: Some(
                "Test this with multi-byte characters: π is 16 bit unicode, 🦽is 32 bit unicode"
                    .to_string(),
            ),
            creation_time: "2022-01-06T13:24:32.73621Z".to_string(),
            _json0: None,
        };
        debug!("Initial contents of struct is {:?}", namespace);

        // Sign the struct
        assert!(namespace.json().is_none());
        namespace
            .sign_json(
                JwsSignatureAlgorithms::RS512,
                &key_pair.private_key,
                &key_pair.public_key,
            )
            .context("Error signing struct")?;
        info!("Signed json {}", namespace.json().unwrap());

        // Verify the signature and check the returned details.
        let attestations = namespace.verify_signature()?;
        assert_eq!(1, attestations.len());
        assert!(attestations[0].signature_is_valid);
        info!("signature is valid");

        let json = namespace.json();
        assert!(json.is_some());

        info!("Creating a copy of the struct by deserializing its JSON");
        let json_string = json.unwrap();
        let mut namespace2: Namespace = Namespace::from_json_string(&json_string).unwrap();

        assert_eq!(namespace, namespace2);
        namespace2.verify_signature()?;

        info!("Clearing JSON and its signatures");
        namespace2.clear_json();
        assert!(namespace2._json0.is_none());

        Ok(())
    }
}
