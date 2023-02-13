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
