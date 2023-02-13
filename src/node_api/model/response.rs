use http::status::StatusCode;
use serde::{Deserialize, Serialize};
use warp::http;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BuildSuccessResponse {
    pub build_id: Option<String>,
    pub message: Option<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub success_status_code: StatusCode,
}
