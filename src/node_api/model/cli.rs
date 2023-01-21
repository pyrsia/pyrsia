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

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Status {
    pub peers_count: usize,
    pub peer_id: String,
    pub peer_addrs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestAddAuthorizedNode {
    pub peer_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestDockerBuild {
    pub image: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransparentLogOutputParams {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestDockerLog {
    pub image: String,
    pub output_params: Option<TransparentLogOutputParams>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestMavenBuild {
    pub gav: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestMavenLog {
    pub gav: String,
    pub output_params: Option<TransparentLogOutputParams>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestBuildStatus {
    pub build_id: String,
}

impl TransparentLogOutputParams {
    pub fn new() -> Self {
        Self { format: None }
    }

    pub fn output_format(params: &Option<Self>) -> Option<&str> {
        if let Some(params) = params {
            return Some(params.format.as_ref().unwrap().as_str());
        }

        Some("json")
    }
}

impl Default for TransparentLogOutputParams {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestDockerLog {
    pub fn output_format(&self) -> Option<&str> {
        TransparentLogOutputParams::output_format(&self.output_params)
    }
}

impl RequestMavenLog {
    pub fn output_format(&self) -> Option<&str> {
        TransparentLogOutputParams::output_format(&self.output_params)
    }
}
