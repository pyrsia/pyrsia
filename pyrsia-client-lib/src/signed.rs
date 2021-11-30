extern crate anyhow;
extern crate detached_jws;
extern crate openssl;
extern crate serde;
extern crate serde_jcs;
extern crate serde_json;

use std::option::Option;

use anyhow::{Context, Result};
use detached_jws::{DeserializeJwsWriter, SerializeJwsWriter};
use openssl::pkey::{PKey, Private, Public};
use openssl::{
    hash::MessageDigest,
    pkey::PKeyRef,
    rsa::{Padding, Rsa},
    sign::{Signer, Verifier},
};
use serde::{Deserialize, Serialize};

/// An enumeration of the supported signature algorithms
#[derive(Deserialize, Serialize)]
pub enum SignatureAlgorithms {
    RsaPkcs1Sha512,
    RsaPkcs1Sha3_512,
}

// The default size for RSA keys
const DEFAULT_RSA_KEY_SIZE: u32 = 4096;

/// An instance of this struct is created to hold a key pair
#[derive(Deserialize, Serialize)]
pub struct SignatureKeyPair {
    pub signature_algorithm: SignatureAlgorithms,
    pub private_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

/// Create and return a key pair using the specified signature algorithm.
pub fn create_key_pair(
    signature_algorithm: SignatureAlgorithms,
) -> Result<SignatureKeyPair, anyhow::Error> {
    match signature_algorithm {
        SignatureAlgorithms::RsaPkcs1Sha3_512 | SignatureAlgorithms::RsaPkcs1Sha512 => {
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
        signature_algorithm: SignatureAlgorithms,
        private_key: &Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let target_json: String = serde_jcs::to_string(self)?;
        with_signer(signature_algorithm, private_key, |signer: Signer, der_public_key: &Vec<u8> | add_signature(signer, der_public_key, target_json))
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

const SIGNATURE_FIELD_NAME: &str = "__signature";

fn with_signer<'a>(
    signature_algorithm: SignatureAlgorithms,
    der_private_key: &[u8],
    signing_function: fn(Signer, &Vec<u8>) -> Result<(), anyhow::Error>,
) -> Result<(), anyhow::Error> {
    let private_key: Rsa<Private> = Rsa::private_key_from_der(der_private_key)?;
    let kp: PKey<Private> = PKey::from_rsa(private_key)?;
    let mut signer = match signature_algorithm {
        SignatureAlgorithms::RsaPkcs1Sha512 => {
            Signer::new(MessageDigest::sha512(), &kp).context("Problem using key pair")
        }
        SignatureAlgorithms::RsaPkcs1Sha3_512 => {
            Signer::new(MessageDigest::sha3_512(), &kp).context("Problem using key pair")
        }
    }?;
    signer.set_rsa_padding(Padding::PKCS1_PSS)?;
    signing_function(signer, &kp.public_key_to_der()?)
}

/// Lightweight JSON parser to identify the portion of a slice before and after a value, so that the
/// value can easily be replaced.
mod json_parser {
    use anyhow::anyhow;
    use serde_json::json;
    use std::str::Chars;

    pub struct JsonCursor<'a> {
        position: usize,
        iterator: Chars<'a>,
        this_char: Option<char>,
        json_str: &'a str,
    }

    impl<'a> JsonCursor<'a> {
        pub fn new(json: &str) -> JsonCursor {
            let mut iterator = json.chars();
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

        fn this_char_equals(&self, c: char) -> bool {
            self.this_char.is_some() && self.this_char.unwrap() == c
        }

        fn expect_char(&mut self, next_char: char) -> Result<(), anyhow::Error> {
            if self.this_char.is_some() && self.this_char.unwrap() == next_char {
                self.next();
                Ok(())
            } else {
                let mut found_char = String::new();
                if self.this_char.is_some() {
                    found_char.push(self.this_char.unwrap())
                } else {
                    found_char.push_str("None")
                }
                Err(anyhow!(format!(
                    "Expected '{}' but found '{}' at position {}.",
                    next_char, found_char, self.position
                )))
            }
        }

        fn char_predicate(&self, predicate: fn(char) -> bool) -> bool {
            self.this_char.is_some() && predicate(self.this_char.unwrap())
        }

        fn at_end(&self) -> bool {
            self.this_char.is_none()
        }

        fn skip_char(&mut self, c: char) {
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
        json: &'a str,
        path: &Vec<JsonPathElement>,
    ) -> Result<(&'a str, &'a str, &'a str), anyhow::Error> {
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
        if end_of_target == 0 && end_of_target <= start_of_target || end_of_target < start_of_target{
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
                Some(field_name),
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
        if json_cursor.this_char_equals('{') {
            parse_object(
                start_of_target,
                end_of_target,
                &mut Vec::new().iter(),
                json_cursor,
                None,
            )
        } else if json_cursor.this_char_equals('[') {
            parse_array(
                start_of_target,
                end_of_target,
                &mut Vec::new().iter(),
                json_cursor,
                None,
            )
        } else if json_cursor.this_char_equals('"'){
            parse_string(json_cursor)?;
            Ok(())
        } else if json_cursor.char_predicate(|c| is_signed_alphanumeric(c)) {
            parse_number_or_id(json_cursor)
        } else {
            Err(anyhow!(format!(
                "Unexpected character '{}' at position {} in json: {}",
                json_cursor.this_char.unwrap_or_default(),
                json_cursor.position,
                json_cursor.json_str
            )))
        }
    }

    fn is_signed_alphanumeric(c: char) -> bool {
        c.is_alphanumeric() || c == '-' || c == '+'
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
        json_cursor.expect_char('[')?;
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
            if json_cursor.this_char_equals(']') {
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
                json_cursor.skip_char(',');
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
                json_cursor.skip_char(',');
            }
            this_index += 1
        }
    }

    fn parse_object(
        start_of_target: &mut usize,
        end_of_target: &mut usize,
        path: &mut core::slice::Iter<JsonPathElement>,
        json_cursor: &mut JsonCursor,
        target_field: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let is_empty_path = path.clone().next().is_none();
        skip_whitespace(json_cursor);
        json_cursor.expect_char('{')?;
        loop {
            let start_position = json_cursor.position;
            skip_whitespace(json_cursor);
            if json_cursor.at_end() {
                return Err(anyhow!(format!(
                    "Unterminated object started at position {}",
                    start_position
                )));
            }
            if json_cursor.this_char_equals('}') {
                if target_field.is_some() && is_empty_path {
                    // path target not found. Pretend we found it at the end of the object as an empty string
                    *start_of_target = start_position;
                    *end_of_target = json_cursor.position;
                }
                json_cursor.next();
                return Ok(());
            };
            let field_name = parse_string(json_cursor)?;
            let field_name2 = String::from(field_name);
            skip_whitespace(json_cursor);
            json_cursor.expect_char(':')?;
            if target_field.unwrap_or_default() == field_name2 {
                parse_value(start_of_target, end_of_target, path, json_cursor)?;
                if is_empty_path {
                    json_cursor.skip_char(',');
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
                json_cursor.skip_char(',')
            }
        }
    }

    pub fn parse_string<'a>(json_cursor: &'a mut JsonCursor) -> Result<&'a str, anyhow::Error> {
        skip_whitespace(json_cursor);
        json_cursor.expect_char('"')?;
        let string_start = json_cursor.position;
        loop {
            if json_cursor.at_end() {
                return Err(anyhow!(format!(
                    "JSON contains an unterminated string that starts at position {}.",
                    string_start
                )));
            }
            if json_cursor.this_char_equals('\\') {
                json_cursor.next(); // Ignore the next character because it is escaped.
            } else if json_cursor.this_char_equals('"') {
                let content = &json_cursor.json_str[string_start..json_cursor.position];
                json_cursor.next();
                return Ok(content);
            }
            json_cursor.next();
        }
    }

    fn skip_whitespace(json_cursor: &mut JsonCursor) {
        while json_cursor.char_predicate(|c| c.is_whitespace()) {
            json_cursor.next()
        }
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
        #[serde(skip)]
        π_json: Option<String>,
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
        let test = |expected_before: &str,
                    expected_middle: &str,
                    expected_after: &str,
                    path: Vec<JsonPathElement>|
         -> Result<(), anyhow::Error> {
            let (actual_before, actual_middle, actual_after) = parse(json, &path)?;
            assert_eq!(expected_before, actual_before);
            assert_eq!(expected_middle, actual_middle);
            assert_eq!(expected_after, actual_after);
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
            vec![JsonPathElement::Field("arr"), JsonPathElement::Index(2), JsonPathElement::Field("sig")],
        )?;
        test(
            r#"{"boo":true,"number":234,"nul":null, "ob":{"a":123,"b":"str"}, "arr":[3, true, {"sig":"mund", "om":"ega""#,
            r#""#,
            r#"}, "asfd"] , "extra":"qwoeiru"}"#,
            vec![JsonPathElement::Field("arr"), JsonPathElement::Index(2), JsonPathElement::Field("Zog")],
        )?;
        Ok(())
    }

    #[test]
    fn parse_string_happy_test() -> Result<(), anyhow::Error> {
        let mut cursor = JsonCursor::new("  \"The quick Brown fox.\" ");
        let parsed_string = parse_string(&mut cursor)?;
        assert_eq!("The quick Brown fox.", parsed_string);
        Ok(())
    }

    #[test]
    fn parse_string_escape() -> Result<(), anyhow::Error> {
        let mut cursor = JsonCursor::new("  \"The quick \\\"Brown\\\" fox.\" ");
        let parsed_string = parse_string(&mut cursor)?;
        assert_eq!("The quick \\\"Brown\\\" fox.", parsed_string);
        Ok(())
    }

    #[test]
    fn parse_string_unterminated() -> Result<(), anyhow::Error> {
        let mut cursor = JsonCursor::new("  \"The quick \\\"Brown\\\" fox. ");
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
            crate::signed::create_key_pair(SignatureAlgorithms::RsaPkcs1Sha3_512)?;

        // create a key pair for other signing types to see that they succeed
        super::create_key_pair(SignatureAlgorithms::RsaPkcs1Sha512)?;

        Ok(())
    }
}
