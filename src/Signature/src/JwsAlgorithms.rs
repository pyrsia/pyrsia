extern crate serde;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq)]
pub enum JwsSignatureAlgorithms {
    RS512,
    RS384,
}
