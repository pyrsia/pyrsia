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
    BlobDoesNotExist,
    ManifestUnknown,
    Unknown(String),
}

impl fmt::Display for RegistryErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match &self {
            RegistryErrorCode::BlobUnknown => "BLOB_UNKNOWN".to_string(),
            RegistryErrorCode::BlobDoesNotExist => "BLOB_DOES_NOT_EXIST".to_string(),
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
            RegistryErrorCode::BlobDoesNotExist => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::BlobDoesNotExist;
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
