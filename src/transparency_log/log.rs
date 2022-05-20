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

use crate::util::env_util::read_var;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, strum_macros::Display, Deserialize, Serialize)]
enum Operation {
    AddArtifact,
}

#[derive(Debug, Deserialize, Serialize)]
struct Payload {
    id: String,
    hash: String,
    timestamp: u64,
    operator: Operation,
}

#[derive(Clone)]
pub struct TransparencyLog {
    payloads: HashMap<String, String>,
}

impl TransparencyLog {
    pub fn new() -> Self {
        TransparencyLog {
            payloads: HashMap::new(),
        }
    }

    pub fn add_artifact(&mut self, id: &str, hash: &str) -> anyhow::Result<()> {
        let payload = Payload {
            id: id.to_string(),
            hash: hash.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operator: Operation::AddArtifact,
        };

        let json_payload = write_payload(&payload)?;
        self.payloads.insert(id.to_string(), json_payload);

        Ok(())
    }
}

fn write_payload(payload: &Payload) -> anyhow::Result<String> {
    let payload_storage_path = get_payload_storage_path();
    fs::create_dir_all(&payload_storage_path)?;
    let payload_filename = format!(
        "{}/{}.log",
        payload_storage_path,
        str::replace(&payload.id, "/", "_")
    );
    debug!(
        "Storing transparency log payload at: {:?}",
        payload_filename
    );
    let mut payload_file = fs::File::create(&payload_filename)?;
    let json_payload = serde_json::to_string(payload)?;
    payload_file.write_all(json_payload.as_bytes())?;
    Ok(json_payload)
}

fn get_payload_storage_path() -> String {
    format!(
        "{}/{}",
        read_var("PYRSIA_ARTIFACT_PATH", "pyrsia"),
        "transparency_log"
    )
}

impl Default for TransparencyLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;
    use std::env;
    use std::path::Path;

    fn tear_down() {
        if Path::new(&env::var("PYRSIA_ARTIFACT_PATH").unwrap()).exists() {
            fs::remove_dir_all(env::var("PYRSIA_ARTIFACT_PATH").unwrap()).expect(&format!(
                "unable to remove test directory {}",
                env::var("PYRSIA_ARTIFACT_PATH").unwrap()
            ));
        }
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_add_artifact() {
        let mut log = TransparencyLog::new();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        assert!(log.payloads.contains_key("id"));
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_add_artifact_with_id_containing_forward_slash() {
        let mut log = TransparencyLog::new();

        let result = log.add_artifact("id/with/slash", "hash");
        assert!(result.is_ok());

        assert!(log.payloads.contains_key("id/with/slash"));
    }
}
