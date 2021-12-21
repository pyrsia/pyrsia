use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]

pub struct Status {
    pub peers_count: usize,
    pub artifact_count: usize,
    pub disk_space_available: String,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Connected Peers Count:   {}", self.peers_count);
        writeln!(f, "Artifacts Count:         {}", self.artifact_count);
        write!(f, "Total Disk Available:    {}", self.disk_space_available)
    }
}
