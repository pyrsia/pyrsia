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

use serde::{Deserialize, Serialize};
use openssl::hash::MessageDigest;

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
    pub fn as_message_digest(&self) -> MessageDigest {
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
