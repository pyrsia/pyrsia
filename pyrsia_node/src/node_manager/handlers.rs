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

use super::ArtifactManager;
use super::HashAlgorithm;

use super::Hash;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::info;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::str;

const ART_MGR_DIR: &str = "pyrsia";

lazy_static! {
    static ref ART_MGR: ArtifactManager = {
        fs::create_dir_all(ART_MGR_DIR).expect("Error creating dir for artifacts");
        ArtifactManager::new(ART_MGR_DIR).unwrap()
    };
}

//get_artifact: given artifact_hash(artifactName) pulls artifact for  artifact_manager and
//              returns read object to read the bytes of artifact
pub fn get_artifact(art_hash: &[u8], art_algorithm: HashAlgorithm) -> Result<File, anyhow::Error> {
    let hash = Hash::new(art_algorithm, art_hash)?;
    ART_MGR
        .pull_artifact(&hash)
        .context("Error from get_artifact")
}

//put_artifact: given artifact_hash(artifactName) & artifact_path push artifact to artifact_manager
//              and returns the boolean as true or false if it was able to create or not
pub fn put_artifact(artifact_hash: &[u8], artifact_path: &str) -> Result<bool, anyhow::Error> {
    let hash = Hash::new(HashAlgorithm::SHA256, artifact_hash)?;
    let file =
        File::open(artifact_path).with_context(|| format!("{} not found.", artifact_path))?;
    let mut buf_reader = BufReader::new(file);

    ART_MGR
        .push_artifact(&mut buf_reader, &hash)
        .context("Error from put_artifact")
}

pub fn get_arts_count() -> Result<usize, anyhow::Error> {
    info!("get_pyrsia_status started");

    ART_MGR
        .artifacts_count(ART_MGR_DIR)
        .context("Error while getting artifacts count")
}

#[cfg(test)]

mod tests {
    use super::*;
    use anyhow::Context;
    use std::env;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::{fs, path::PathBuf};

    const GOOD_ART_HASH: [u8; 32] = [
        0x66, 0xdc, 0x9a, 0xc8, 0xb2, 0x77, 0x12, 0xbc, 0x2c, 0x5d, 0xa3, 0x61, 0xab, 0x41, 0x75,
        0x20, 0x6e, 0x27, 0x1a, 0x8a, 0x90, 0xd2, 0x1, 0xfb, 0xbe, 0x7, 0xb8, 0x81, 0xed, 0x8e,
        0xec, 0xa7,
    ];

    #[test]
    fn put_and_get_artifact_test() -> Result<(), anyhow::Error> {
        println!("put_and_get_artifact_test started !!");

        // tetst artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("resources/test/artifact_test.txt");
        println!("curr_dir is: {}", curr_dir.display());

        let path = String::from(curr_dir.to_string_lossy());

        //put the artifact
        put_artifact(&GOOD_ART_HASH, path.as_str()).context("Error from put_artifact")?;

        //validate pushed artifact with actual data
        let mut push_art_path = PathBuf::from(ART_MGR_DIR);
        push_art_path.push("SHA256");
        push_art_path.push(hex::encode(GOOD_ART_HASH));
        push_art_path.set_extension("file");
        println!("reading artifact path is: {}", push_art_path.display());
        let content_vec = fs::read(push_art_path.as_path()).context("reading pushed file")?;

        let test_art_path = PathBuf::from(path.as_str());
        let actual_content_vec = fs::read(test_art_path.as_path()).context("reading test file")?;

        assert_eq!(content_vec.as_slice(), actual_content_vec.as_slice());

        // pull artiafct
        let file = get_artifact(&GOOD_ART_HASH, HashAlgorithm::SHA256)
            .context("Error from get_artifact")?;

        //validate pulled artifact with the actual data
        let mut buf_reader = BufReader::new(file);
        let mut content = String::new();
        buf_reader.read_to_string(&mut content)?;

        let s = match str::from_utf8(actual_content_vec.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        assert_eq!(s, content);

        //cleaning up
        fs::remove_dir_all(ART_MGR_DIR)
            .context(format!("Error cleaning up directory {}", path.clone()))?;

        println!("put_and_get_artifact_test ended !!");
        Ok(())
    }
}
