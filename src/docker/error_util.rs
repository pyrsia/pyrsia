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

use log::debug;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::fmt;
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{Rejection, Reply};

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorMessage {
    code: RegistryErrorCode,
    message: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorMessages {
    errors: Vec<ErrorMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum RegistryErrorCode {
    BlobUnknown,
    BlobDoesNotExist(String),
    ManifestUnknown,
    Unknown(String),
}

impl fmt::Display for RegistryErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match &self {
            RegistryErrorCode::BlobUnknown => "BLOB_UNKNOWN".to_string(),
            RegistryErrorCode::BlobDoesNotExist(hash) => format!("BLOB_DOES_NOT_EXIST({})", hash),
            RegistryErrorCode::ManifestUnknown => "MANIFEST_UNKNOWN".to_string(),
            RegistryErrorCode::Unknown(m) => format!("UNKNOWN({})", m),
        };
        write!(f, "{}", printable)
    }
}

#[derive(Debug)]
pub struct RegistryError {
    pub code: RegistryErrorCode,
}

impl Reject for RegistryError {}

pub async fn custom_recover(err: Rejection) -> Result<impl Reply, Infallible> {
    let mut status_code = StatusCode::INTERNAL_SERVER_ERROR;
    let mut error_message = ErrorMessage {
        code: RegistryErrorCode::Unknown("".to_string()),
        message: "".to_string(),
    };

    debug!("Rejection: {:?}", err);
    if let Some(e) = err.find::<RegistryError>() {
        match &e.code {
            RegistryErrorCode::BlobUnknown => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::BlobUnknown;
            }
            RegistryErrorCode::BlobDoesNotExist(hash) => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::BlobDoesNotExist(hash.to_string());
            }
            RegistryErrorCode::ManifestUnknown => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::ManifestUnknown;
            }
            RegistryErrorCode::Unknown(m) => {
                error_message.message = m.clone();
            }
        }
    } else if let Some(e) = err.find::<warp::reject::InvalidHeader>() {
        status_code = StatusCode::BAD_REQUEST;
        error_message.message = format!("{}", e);
    }

    debug!("ErrorMessage: {:?}", error_message);
    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorMessages {
            errors: vec![error_message],
        }),
        status_code,
    )
    .into_response())
}
