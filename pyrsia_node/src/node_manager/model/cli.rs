
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize)]

pub struct Status {
    pub peers_count: usize,
    pub artifact_count: usize,
    pub disk_space_available: String,

}