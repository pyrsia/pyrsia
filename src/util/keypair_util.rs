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
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use libp2p::identity;
use log::{error, warn};
use std::error;
use std::fs;
use std::io::{self, Read, Write};
use std::panic::UnwindSafe;
use std::path::Path;

lazy_static! {
    pub static ref KEYPAIR_FILENAME: String = {
        let pyrsia_keypair_file = read_var("PYRSIA_KEYPAIR", "pyrsia/p2p_keypair.ser");
        let pyrsia_keypair_path = Path::new(&pyrsia_keypair_file);
        log_static_initialization_failure(
            "Pyrsia Key Pair directory",
            std::fs::create_dir_all(pyrsia_keypair_path.parent().unwrap()).with_context(|| {
                format!(
                    "Failed to create key pair directory {:?}",
                    pyrsia_keypair_path.parent()
                )
            }),
        );
        pyrsia_keypair_path
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string()
    };
}

fn log_static_initialization_failure<T: UnwindSafe>(
    label: &str,
    result: Result<T, anyhow::Error>,
) -> T {
    let panic_wrapper = std::panic::catch_unwind(|| match result {
        Ok(unwrapped) => unwrapped,
        Err(error) => {
            let msg = format!("Error initializing {}, error is: {}", label, error);
            error!("{}", msg);
            panic!("{}", msg)
        }
    });
    match panic_wrapper {
        Ok(normal) => normal,
        Err(partially_unwound_panic) => {
            error!("Initialization of {} panicked!", label);
            std::panic::resume_unwind(partially_unwound_panic)
        }
    }
}

/// Load a ed25519 keypair from disk. If a keypair file does not yet exist,
/// a new keypair is generated and then saved to disk.
pub fn load_or_generate_ed25519<P: AsRef<Path>>(storage_path: P) -> identity::Keypair {
    let keypair_path = storage_path.as_ref();
    match load_ed25519(keypair_path) {
        Ok(keypair) => identity::Keypair::Ed25519(keypair),
        Err(_) => {
            let keypair = identity::ed25519::Keypair::generate();
            if let Err(e) = save_ed25519(&keypair, keypair_path) {
                warn!(
                    "Failed to persist newly generated keypair: {} {:?}",
                    keypair_path.display(),
                    e
                );
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
fn load_ed25519(keypair_path: &Path) -> Result<identity::ed25519::Keypair, io::Error> {
    let mut keypair_file = fs::File::open(keypair_path)?;
    let keypair_metadata = fs::metadata(keypair_path)?;
    if keypair_metadata.len() == 64 {
        let mut buffer = vec![0; 64];
        keypair_file.read_exact(&mut buffer)?;
        if let Ok(keypair) = identity::ed25519::Keypair::decode(&mut buffer) {
            return Ok(keypair);
        }
    }

    Err(io::Error::from(io::ErrorKind::InvalidData))
}

// Save the provided keypair to the specified path.
fn save_ed25519(
    keypair: &identity::ed25519::Keypair,
    keypair_path: &Path,
) -> Result<(), Box<dyn error::Error>> {
    let parent = keypair_path.parent().unwrap();
    std::fs::create_dir_all(parent)?;
    let mut keypair_file = fs::File::create(keypair_path)?;
    keypair_file.write_all(&keypair.encode())?;
    Ok(())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_non_existing_keypair_generates_new_keypair_and_saves_it() {
        let tmp_dir = tempfile::tempdir().unwrap().path().join("p2p_keypair.ser");
        let tmp_keypair = tmp_dir.as_path();

        load_or_generate_ed25519(tmp_keypair);
        assert!(tmp_keypair.exists());
    }

    #[test]
    fn keypair_generates_new_keypair_but_does_not_save_it() {
        let tmp_dir = if cfg!(target_os = "windows") {
            PathBuf::from("AB:\\remarkable")
        } else {
            let tmp_dir = tempfile::tempdir().unwrap().into_path();
            let mut perms = fs::metadata(&tmp_dir).unwrap().permissions();
            perms.set_readonly(true);
            fs::set_permissions(&tmp_dir, perms).unwrap();
            tmp_dir
        };
        let tmp_file = tmp_dir.join("keypair_fails");
        load_or_generate_ed25519(tmp_file.as_path());
        assert!(!tmp_file.as_path().exists());
    }

    #[test]
    fn load_existing_keypair_with_wrong_size_fails() {
        let tmp_file = tempfile::Builder::new().tempfile().unwrap();
        tmp_file.as_file().write_all(&[1; 32]).unwrap();

        let keypair = load_ed25519(tmp_file.path());
        assert_eq!(keypair.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn saved_keypair_can_be_loaded() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("p2p_keypair.ser");

        let saved_keypair = identity::ed25519::Keypair::generate();
        let save_result = save_ed25519(&saved_keypair, &path);
        assert!(save_result.is_ok());

        let loaded_keypair = load_or_generate_ed25519(&path);
        assert_eq!(
            identity::Keypair::Ed25519(saved_keypair)
                .to_protobuf_encoding()
                .unwrap(),
            loaded_keypair.to_protobuf_encoding().unwrap()
        );
    }
}
