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
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    pub peers_count: usize,
    pub peer_id: String,
    pub artifact_count: ArtifactsSummary,
    pub disk_allocated: String,
    pub disk_usage: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ArtifactsSummary {
    pub total: String,
    pub summary: HashMap<String, usize>,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Connected Peers Count:       {}", self.peers_count)?;
        writeln!(f, "Artifacts Count:             {}", self.artifact_count)?;
        writeln!(f, "Total Disk Space Allocated:  {}", self.disk_allocated)?;
        write!(f, "Disk Space Used:             {}%", self.disk_usage)
    }
}

impl std::fmt::Display for ArtifactsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {:?}", self.total, self.summary)
    }
}
