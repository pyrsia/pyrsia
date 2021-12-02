extern crate anyhow;
extern crate base64;
extern crate chrono;
extern crate detached_jws;
extern crate openssl;
extern crate serde;
extern crate serde_jcs;
extern crate serde_json;

use std::char::REPLACEMENT_CHARACTER;
use std::io::Write;
use std::option::Option;

use anyhow::{Context, Result};
use chrono::prelude::*;
use detached_jws::{DeserializeJwsWriter, SerializeJwsWriter};
use openssl::pkey::{PKey, Private, Public};
use openssl::{
    hash::MessageDigest,
    pkey::PKeyRef,
    rsa::{Padding, Rsa},
    sign::{Signer, Verifier},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// An enumeration of the supported signature algorithms
#[derive(Deserialize, Serialize)]
pub enum JwsSignatureAlgorithms {
    RS512,
    RS384,
}

impl JwsSignatureAlgorithms {
    pub fn to_string(&self) -> String {
        String::from(match self {
            JwsSignatureAlgorithms::RS512 => "RS512",
            JwsSignatureAlgorithms::RS384 => "RS384",
        })
    }
}

// The default size for RSA keys
const DEFAULT_RSA_KEY_SIZE: u32 = 4096;

/// An instance of this struct is created to hold a key pair
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
/// //noinspection NonAsciiCharacters
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Foo<'a> {
///   foo: &'a str,
///   bar: u32,
///   #[serde(skip)]
///   π_json: Option<String>
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
    /// * signature_algorithm — The signature algorithm to use for signing. Must be compatible with the private key.
    /// * private_key — The der encoded private key to use for signing.
    fn sign(
        &mut self,
        signature_algorithm: JwsSignatureAlgorithms,
        private_key: &Vec<u8>,
        public_key: &Vec<u8>,
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

    /// Verify the signature(s) of this struct's associated JSON.kp
    ///
    /// Return an error if any of the signatures are not valid.
    fn verify_signature(&self) -> Result<(), anyhow::Error> {
        todo!()
    }

    // TODO add a method to get the details of the signatures in this struct's associated JSON.
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

// construct a signer, pass it to the given signing function and then return the signed json returned from the signing function.
fn with_signer<'a>(
    signature_algorithm: JwsSignatureAlgorithms,
    der_private_key: &[u8],
    der_public_key: &[u8],
    target_json: &[u32],
    signing_function: fn(
        JwsSignatureAlgorithms,
        Signer,
        &[u8],
        &[u32],
    ) -> Result<String, anyhow::Error>,
) -> Result<String, anyhow::Error> {
    // This is RSA specific. This should be generalized to support other types of signatures.
    let private_key: Rsa<Private> = Rsa::private_key_from_der(der_private_key)?;
    let kp: PKey<Private> = PKey::from_rsa(private_key)?;
    let mut signer = match signature_algorithm {
        JwsSignatureAlgorithms::RS512 => {
            Signer::new(MessageDigest::sha512(), &kp).context("Problem using key pair")
        }
        JwsSignatureAlgorithms::RS384 => {
            Signer::new(MessageDigest::sha384(), &kp).context("Problem using key pair")
        }
    }?;
    signer.set_rsa_padding(Padding::PKCS1_PSS)?;
    signing_function(signature_algorithm, signer, der_public_key, target_json)
}

fn add_signature<'a>(
    signature_algorithm: JwsSignatureAlgorithms,
    signer: Signer,
    der_public_key: &[u8],
    target_json: &[u32],
) -> Result<String, anyhow::Error> {
    let (before, middle, after) = json_parser::parse(
        target_json,
        &vec![json_parser::JsonPathElement::Field(SIGNATURE_FIELD_NAME)],
    )?;
    let header = create_jsw_header(der_public_key);
    let before_string = unicode_32_bit_to_string(before);
    let after_string = unicode_32_bit_to_string(after);
    let jws = create_jws(
        signature_algorithm,
        signer,
        &before_string,
        &after_string,
        header,
    )?;
    let jws_string = String::from_utf8(jws)?;
    let mut signed_json_buffer = String::from(before_string);
    if middle.is_empty() {
        // No existing signatures
        signed_json_buffer.push_str(",\"");
        signed_json_buffer.push_str(SIGNATURE_FIELD_NAME);
        signed_json_buffer.push_str(r#"":[""#);
        signed_json_buffer.push_str(jws_string.as_str());
        signed_json_buffer.push_str("\"]");
    } else {
        signed_json_buffer.push_str(unicode_32_bit_to_string(&middle[..middle.len() - 1]).as_str()); // append signature array without closing ']'
        signed_json_buffer.push_str(",\"");
        signed_json_buffer.push_str(jws_string.as_str());
        signed_json_buffer.push_str("\"]");
    }
    signed_json_buffer.push_str(&after_string);
    Ok(String::from(signed_json_buffer))
}

fn create_jws(
    signature_algorithm: JwsSignatureAlgorithms,
    signer: Signer,
    before: &str,
    after: &str,
    header: Map<String, Value>,
) -> Result<Vec<u8>, anyhow::Error> {
    let mut writer =
        SerializeJwsWriter::new(Vec::new(), signature_algorithm.to_string(), header, signer)?;
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

fn create_jsw_header(public_key: &[u8]) -> Map<String, Value> {
    let mut header = Map::new();
    header.insert(
        "signer".to_owned(),
        json!(base64::encode_config(public_key, base64::STANDARD_NO_PAD)),
    );
    header.insert("timestamp".to_owned(), json!(format!("{:?}", Utc::now())));
    header
}

/// Lightweight JSON parser to identify the portion of a slice before and after a value, so that the
/// value can easily be replaced.
mod json_parser {
    use super::string_to_unicode_32;
    use crate::signed::unicode_32_bit_to_string;
    use anyhow::anyhow;
    use serde_json::json;
    use std::char::REPLACEMENT_CHARACTER;
    use std::slice::Iter;
    use std::str::Chars;

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

    pub enum JsonPathElement<'a> {
        Field(&'a str),
        Index(usize),
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
        path: &Vec<JsonPathElement>,
    ) -> Result<(&'a [u32], &'a [u32], &'a [u32]), anyhow::Error> {
        if path.is_empty() {
            return Err(anyhow!("Empty path; nothing to find"));
        }
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
            return Err(anyhow!(format!("Did not find {}", path_to_str(path))));
        }
        Ok((
            &json[..(start_of_target)],
            &json[start_of_target..end_of_target],
            &json[end_of_target..],
        ))
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
        } else if json_cursor.char_predicate(|c| is_signed_alphanumeric(c)) {
            parse_number_or_id(json_cursor)
        } else {
            Err(anyhow!(format!(
                "Unexpected character '{}' at position {} in json: {}",
                &json_cursor.this_char.map_or(String::from("None"), |c| String::from(
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
        while json_cursor.char_predicate(|c| is_signed_alphanumeric(c)) {
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

    pub fn parse_string<'a>(json_cursor: & mut JsonCursor) -> Result<Vec<u32>, anyhow::Error> {
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
        while json_cursor.char_predicate(|c| is_whitespace(c)) {
            json_cursor.next()
        }
    }

    fn is_whitespace(u: u32) -> bool {
        u == 0x09 || u == 0x0a || u == 0x0d || u == 0x20 || u == 0x00a0 || u == 0x1680
            || u == 0x180e || (u >= 0x2000  && u <= 0x200b) || u == 0x202f || u == 0x205f
            || u == 0x3000 || u==0xfeff
    }

    pub fn path_to_str(path: &Vec<JsonPathElement>) -> String {
        let mut s = String::from("path[");
        if !path.is_empty() {
            path_element_to_str(&mut s, &path[0]);
            for path_element in path[1..].iter() {
                s.push_str("\",");
                path_element_to_str(&mut s, path_element);
            }
        }
        s.push_str("]");
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

    //noinspection NonAsciiCharacters
    #[derive(Serialize, Deserialize)]
    struct Foo<'a> {
        foo: &'a str,
        bar: u32,
        zot: &'a str,
        #[serde(skip)]
        π_json: Option<String>,
    }

    impl<'a> Signed<'a> for Foo<'a> {
        fn json(&self) -> Option<String> {
            self.π_json.to_owned()
        }

        fn clear_json(&mut self) {
            self.π_json = None;
        }

        fn set_json(&mut self, json: &str) {
            self.π_json = Option::Some(json.to_string())
        }
    }

    #[test]
    fn path_to_string_test() {
        let path = vec![
            JsonPathElement::Field("__signature"),
            JsonPathElement::Index(4),
        ];
        assert_eq!("path[field:\"__signature\",4]", path_to_str(&path))
    }

    #[test]
    fn parse_json() -> Result<(), anyhow::Error> {
        let json = r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega"}, "asfd"] , "extra":"qwoeiru"}"#;
        let json32 = string_to_unicode_32(json);
        let test = |expected_before: &str,
                    expected_middle: &str,
                    expected_after: &str,
                    path: Vec<JsonPathElement>|
         -> Result<(), anyhow::Error> {
            let (actual_before, actual_middle, actual_after) =
                parse(&json32, &path)?;

            assert_eq!(expected_before, unicode_32_bit_to_string(actual_before));
            assert_eq!(expected_middle, unicode_32_bit_to_string(actual_middle));
            assert_eq!(expected_after, unicode_32_bit_to_string(actual_after));
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
        let test32 = string_to_unicode_32("  \"The quick Brown fox.\" ");
        let mut cursor = JsonCursor::new(&test32);
        let parsed_string = parse_string(&mut cursor)?;
        let expected = string_to_unicode_32("The quick Brown fox.");
        assert_eq!(expected, parsed_string);
        Ok(())
    }

    #[test]
    fn parse_string_escape() -> Result<(), anyhow::Error> {
        let test32 = string_to_unicode_32("  \"The quick \\\"Brown\\\" fox.\" ");
        let mut cursor = JsonCursor::new(&test32);
        let parsed_string = parse_string(&mut cursor)?;
        let expected = string_to_unicode_32(r#"The quick \"Brown\" fox."#);
        assert_eq!(expected, parsed_string);
        Ok(())
    }

    #[test]
    fn parse_string_unterminated() -> Result<(), anyhow::Error> {
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
    fn happy_path_for_signing() -> Result<(), anyhow::Error> {
        let key_pair: SignatureKeyPair =
            crate::signed::create_key_pair(JwsSignatureAlgorithms::RS512)?;

        // create a key pair for other signing types to see that they succeed
        let key_pair = super::create_key_pair(JwsSignatureAlgorithms::RS512)?;

        let mut foo = Foo {
            foo: "π is 16 bit unicode",
            bar: 23894,
            zot: "🦽is 32 bit unicode",
            π_json: None,
        };
        foo.sign(
            JwsSignatureAlgorithms::RS512,
            &key_pair.private_key,
            &key_pair.public_key,
        )
        .context("Error signing struct")?;
        println!("Signed json from foo {}", foo.json().unwrap());
        Ok(())
    }
}
