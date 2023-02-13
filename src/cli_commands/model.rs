use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BuildResultResponse {
    pub build_id: Option<String>,
    pub message: Option<String>,
}
