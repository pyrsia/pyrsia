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

use super::JwsSignatureAlgorithms;

use time::OffsetDateTime;
use serde_json::{json, Map, Value};
use log::warn;

const SIGNATURE_FIELD_NAME: &str = "__signature";
const SIGNER_FIELD_NAME: &str = "signer";
const ALG_FIELD_NAME: &str = "alg";
const TIMESTAMP_FIELD_NAME: &str = "timestamp";
const EXPIRATION_FIELD_NAME: &str = "ext";

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
    timestamp: Option<OffsetDateTime>,
    expiration_time: Option<OffsetDateTime>,
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
    pub fn timestamp(&self) -> &Option<OffsetDateTime> {
        &self.timestamp
    }

    /// The optional expiration time of the signature
    pub fn expiration_time(&self) -> &Option<OffsetDateTime> {
        &self.expiration_time
    }

    /// True if signature verification determined that the signature is valid.
    pub fn signature_is_valid(&self) -> bool {
        self.signature_is_valid
    }

    // create an attestation with all the information from the JWS header.
    // The is_valid field is set to false. It is the responsibility of the caller to change it if valid.
    pub fn from_json(jws_header: &str) -> Attestation {
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

fn date_time_from_json(json_header: &Value, field_name: &str) -> Option<OffsetDateTime> {
    match &json_header[field_name] {
        Value::String(time_string) => {
            let unquoted_time_string: &str = time_string[1..time_string.len() - 1].as_ref();
            parse_iso8601(unquoted_time_string)
        }
        _ => None,
    }
}

fn parse_iso8601(dt_string: &str) -> Option<OffsetDateTime> {
    match iso8601::datetime(dt_string) {
        Ok(date_time) => {
            match date_time.date {
                iso8601::Date::YMD { year, month, day } => Some(
                    iso8601_date_time_to_offset_date_time(year, month, day, date_time.time),
                ),
                iso8601::Date::Week {
                    year: _,
                    ww: _,
                    d: _,
                } => {
                    warn!(
                        "Unsupported timestamp in year-week-day format {}",
                        dt_string
                    );
                    None
                }
                iso8601::Date::Ordinal { year: _, ddd: _ } => {
                    warn!("Unsupported timestamp in year-day format {}", dt_string);
                    None
                }
            }
        }
        Err(error) => {
            warn!("Error parsing JSON timestamp {}", error);
            None
        }
    }
}

fn iso8601_date_time_to_offset_date_time(
    year: i32,
    month: u32,
    day: u32,
    time: iso8601::Time,
) -> OffsetDateTime {
    let formatted_date_time = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}:{:03} {:02}:{:02}",
        year,
        month,
        day,
        time.hour,
        time.minute,
        time.second,
        time.millisecond,
        time.tz_offset_hours,
        time.tz_offset_minutes
    );
    println!("{}", formatted_date_time);
    let format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second]:[subsecond] [offset_hour]:[offset_minute]",
    )
    .unwrap();
    OffsetDateTime::parse(&formatted_date_time, &format).unwrap()
}
