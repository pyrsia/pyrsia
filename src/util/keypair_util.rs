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

use libp2p::identity;
use log::warn;
use std::error;
use std::fs;
use std::io::{self, Read, Write};

const KEYPAIR_FILENAME: &str = "p2p_keypair.ser";

/// Load a ed25519 keypair from disk. If a keypair file does not yet exist,
/// a new keypair is generated and then saved to disk.
pub fn load_or_generate_ed25519() -> identity::Keypair {
    let keypair_path = get_keypair_path();
    match load_ed25519(&keypair_path) {
        Ok(keypair) => identity::Keypair::Ed25519(keypair),
        Err(_) => {
            let keypair = identity::ed25519::Keypair::generate();
            if let Err(e) = save_ed25519(&keypair, &keypair_path) {
                warn!("Failed to persist newly generated keypair: {:?}", e);
            }
            identity::Keypair::Ed25519(keypair)
        }
    }
}

// Load a keypair from the specified path. It only returns a Keypair if all
// the following conditions are met:
//
//  * the file at the specified path exists
//  * the size of the file is exactly 64 bytes
//  * no io errors occured while reading from the file
fn load_ed25519(keypair_path: &str) -> Result<identity::ed25519::Keypair, Box<dyn error::Error>> {
    let mut keypair_file = fs::File::open(keypair_path)?;
    let keypair_metadata = fs::metadata(keypair_path)?;
    if keypair_metadata.len() == 64 {
        let mut buffer = vec![0; 64];
        keypair_file.read_exact(&mut buffer)?;
        return Ok(identity::ed25519::Keypair::decode(&mut buffer)?);
    }

    Err(Box::new(io::Error::from(io::ErrorKind::InvalidData)))
}

// Save the provided keypair to the specified path.
fn save_ed25519(
    keypair: &identity::ed25519::Keypair,
    keypair_path: &str,
) -> Result<(), Box<dyn error::Error>> {
    let mut keypair_file = fs::File::create(&keypair_path)?;
    keypair_file.write_all(&keypair.encode())?;
    Ok(())
}

// Get the path on disk where the keypair is stored.
fn get_keypair_path() -> String {
    format!(
        "{}/{}",
        read_var("PYRSIA_ARTIFACT_PATH", "pyrsia"),
        KEYPAIR_FILENAME
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_non_existing_keypair_fails() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir
            .path()
            .join("load_non_existing_keypair_fails")
            .join(KEYPAIR_FILENAME);

        let keypair = load_ed25519(path.to_str().unwrap());
        assert!(keypair.is_err());
    }

    #[test]
    fn load_existing_keypair_with_wrong_size_fails() {
        let tmp_file = tempfile::Builder::new().tempfile().unwrap();
        tmp_file.as_file().write_all(&[1; 32]).unwrap();

        let keypair = load_ed25519(tmp_file.path().to_str().unwrap());
        assert!(keypair.is_err());
    }

    #[test]
    fn load_existing_keypair_succeeds() {
        let tmp_file = tempfile::Builder::new().tempfile().unwrap();
        tmp_file.as_file().write_all(&[1; 64]).unwrap();

        let keypair = load_ed25519(tmp_file.path().to_str().unwrap());
        assert!(keypair.is_ok());
    }

    #[test]
    fn saved_keypair_can_be_loaded() {
        let tmp_file = tempfile::Builder::new().tempfile().unwrap();

        let saved_keypair = identity::ed25519::Keypair::generate();
        let save_result = save_ed25519(&saved_keypair, tmp_file.path().to_str().unwrap());
        assert!(save_result.is_ok());

        let loaded_keypair = load_ed25519(tmp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(saved_keypair.encode(), loaded_keypair.encode());
    }
}
