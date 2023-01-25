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

use crate::build_service::error::BuildError;
use crate::transparency_log::log::TransparencyLogError;
use log::debug;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::error::Error;
use std::string::FromUtf8Error;
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

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum RegistryErrorCode {
    BlobUnknown,
    ManifestUnknown,
    BadRequest(String),
    Unknown(String),
}

#[derive(Debug, Eq, PartialEq)]
pub struct RegistryError {
    pub code: RegistryErrorCode,
}

impl From<anyhow::Error> for RegistryError {
    fn from(err: anyhow::Error) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<BuildError> for RegistryError {
    fn from(err: BuildError) -> RegistryError {
        match err {
            BuildError::ArtifactAlreadyExists(_) => RegistryError {
                code: RegistryErrorCode::BadRequest(err.to_string()),
            },
            _ => RegistryError {
                code: RegistryErrorCode::Unknown(err.to_string()),
            },
        }
    }
}

impl From<TransparencyLogError> for RegistryError {
    fn from(err: TransparencyLogError) -> RegistryError {
        match err {
            TransparencyLogError::NodeAlreadyExists { .. } => RegistryError {
                code: RegistryErrorCode::BadRequest(err.to_string()),
            },
            _ => RegistryError {
                code: RegistryErrorCode::Unknown(err.to_string()),
            },
        }
    }
}

impl From<hex::FromHexError> for RegistryError {
    fn from(err: hex::FromHexError) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<reqwest::Error> for RegistryError {
    fn from(err: reqwest::Error) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(err: serde_json::Error) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(err: std::io::Error) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<Box<dyn Error>> for RegistryError {
    fn from(err: Box<dyn Error>) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<Box<dyn Error + Send>> for RegistryError {
    fn from(err: Box<dyn Error + Send>) -> RegistryError {
        RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        }
    }
}

impl From<csv::Error> for RegistryError {
    fn from(value: csv::Error) -> Self {
        RegistryError {
            code: RegistryErrorCode::Unknown(value.to_string()),
        }
    }
}

impl From<csv::IntoInnerError<csv::Writer<Vec<u8>>>> for RegistryError {
    fn from(value: csv::IntoInnerError<csv::Writer<Vec<u8>>>) -> Self {
        RegistryError {
            code: RegistryErrorCode::Unknown(value.to_string()),
        }
    }
}

impl From<FromUtf8Error> for RegistryError {
    fn from(value: FromUtf8Error) -> Self {
        RegistryError {
            code: RegistryErrorCode::Unknown(value.to_string()),
        }
    }
}

impl From<hyper::http::Error> for RegistryError {
    fn from(value: hyper::http::Error) -> Self {
        RegistryError {
            code: RegistryErrorCode::Unknown(value.to_string()),
        }
    }
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
            RegistryErrorCode::ManifestUnknown => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::ManifestUnknown;
            }
            RegistryErrorCode::BadRequest(m) => {
                status_code = StatusCode::BAD_REQUEST;
                error_message.code = RegistryErrorCode::BadRequest(m.clone());
                error_message.message = m.clone();
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

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::model::PackageType;
    use std::io;
    use std::str;
    use warp::reply::Response;

    #[test]
    fn from_io_error() {
        let io_error_1 = io::Error::new(io::ErrorKind::Interrupted, "operation interrupted");
        let io_error_2 = io::Error::new(io::ErrorKind::Interrupted, "operation interrupted");

        let registry_error: RegistryError = io_error_1.into();
        assert_eq!(
            registry_error.code,
            RegistryErrorCode::Unknown(io_error_2.to_string())
        );
    }

    #[test]
    fn from_from_hex_error() {
        let from_hex_error = hex::FromHexError::OddLength;

        let registry_error: RegistryError = from_hex_error.into();
        assert_eq!(
            registry_error.code,
            RegistryErrorCode::Unknown(from_hex_error.to_string())
        );
    }

    #[test]
    fn from_anyhow_error() {
        let from_hex_error_1 = hex::FromHexError::OddLength;
        let anyhow_error_1: anyhow::Error = from_hex_error_1.into();

        let from_hex_error_2 = hex::FromHexError::OddLength;
        let anyhow_error_2: anyhow::Error = from_hex_error_2.into();

        let registry_error: RegistryError = anyhow_error_1.into();
        assert_eq!(
            registry_error.code,
            RegistryErrorCode::Unknown(anyhow_error_2.to_string())
        );
    }

    #[test]
    fn from_build_error() {
        let build_error_1 = BuildError::Failure("build_id".to_owned(), "Failed".to_owned());
        let build_error_2 = BuildError::Failure("build_id".to_owned(), "Failed".to_owned());

        let registry_error: RegistryError = build_error_1.into();
        assert_eq!(
            registry_error.code,
            RegistryErrorCode::Unknown(build_error_2.to_string())
        );
    }

    #[test]
    fn from_transparency_log_error() {
        let transparency_log_error_1 = TransparencyLogError::ArtifactNotFound {
            package_type: PackageType::Docker,
            package_specific_artifact_id: "package_specific_artifact_id".to_owned(),
        };
        let transparency_log_error_2 = TransparencyLogError::ArtifactNotFound {
            package_type: PackageType::Docker,
            package_specific_artifact_id: "package_specific_artifact_id".to_owned(),
        };

        let registry_error: RegistryError = transparency_log_error_1.into();
        assert_eq!(
            registry_error.code,
            RegistryErrorCode::Unknown(transparency_log_error_2.to_string())
        );
    }

    #[tokio::test]
    async fn custom_recover_from_registry_error_for_blob_unknown() {
        let registry_error = RegistryError {
            code: RegistryErrorCode::BlobUnknown,
        };

        let expected_body = serde_json::to_string(&ErrorMessages {
            errors: vec![ErrorMessage {
                code: RegistryErrorCode::BlobUnknown,
                message: "".to_string(),
            }],
        })
        .expect("Generating JSON body should not fail.");

        let response = custom_recover(registry_error.into())
            .await
            .expect("Reply should be created.")
            .into_response();

        verify_recover_response(response, expected_body, StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn custom_recover_from_registry_error_for_manifest_unknown() {
        let registry_error = RegistryError {
            code: RegistryErrorCode::ManifestUnknown,
        };

        let expected_body = serde_json::to_string(&ErrorMessages {
            errors: vec![ErrorMessage {
                code: RegistryErrorCode::ManifestUnknown,
                message: "".to_string(),
            }],
        })
        .expect("Generating JSON body should not fail.");

        let response = custom_recover(registry_error.into())
            .await
            .expect("Reply should be created.")
            .into_response();

        verify_recover_response(response, expected_body, StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn custom_recover_from_registry_error_bad_request() {
        let registry_error = RegistryError {
            code: RegistryErrorCode::BadRequest(String::from("bad_request")),
        };

        let expected_body = serde_json::to_string(&ErrorMessages {
            errors: vec![ErrorMessage {
                code: RegistryErrorCode::BadRequest("bad_request".to_string()),
                message: String::from("bad_request"),
            }],
        })
        .expect("Generating JSON body should not fail.");

        let response = custom_recover(registry_error.into())
            .await
            .expect("Reply should be created.")
            .into_response();

        verify_recover_response(response, expected_body, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn custom_recover_from_registry_error_for_unknown() {
        let registry_error = RegistryError {
            code: RegistryErrorCode::Unknown(String::from("unknown_error")),
        };

        let expected_body = serde_json::to_string(&ErrorMessages {
            errors: vec![ErrorMessage {
                code: RegistryErrorCode::Unknown("".to_string()),
                message: String::from("unknown_error"),
            }],
        })
        .expect("Generating JSON body should not fail.");

        let response = custom_recover(registry_error.into())
            .await
            .expect("Reply should be created.")
            .into_response();

        verify_recover_response(response, expected_body, StatusCode::INTERNAL_SERVER_ERROR).await;
    }

    #[derive(Debug)]
    struct UnhandledErrorForCustomRecover {}
    impl Reject for UnhandledErrorForCustomRecover {}

    #[tokio::test]
    async fn custom_recover_from_registry_error_for_unhandled_error() {
        let unhandled_error = UnhandledErrorForCustomRecover {};

        let expected_body = serde_json::to_string(&ErrorMessages {
            errors: vec![ErrorMessage {
                code: RegistryErrorCode::Unknown("".to_string()),
                message: String::from(""),
            }],
        })
        .expect("Generating JSON body should not fail.");

        let response = custom_recover(unhandled_error.into())
            .await
            .expect("Reply should be created.")
            .into_response();

        verify_recover_response(response, expected_body, StatusCode::INTERNAL_SERVER_ERROR).await;
    }

    async fn verify_recover_response(
        response: Response,
        expected_body: String,
        expected_status: StatusCode,
    ) {
        let status = response.status();
        let actual_body_bytes = hyper::body::to_bytes(response.into_body())
            .await
            .expect("Response body to be converted to bytes");
        let actual_body_str = str::from_utf8(&actual_body_bytes)
            .map(str::to_owned)
            .expect("Response body to be converted to string");
        assert_eq!(status, expected_status);
        assert_eq!(actual_body_str, expected_body);
    }
}
