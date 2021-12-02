use super::ArtifactManager;
use super::HashAlgorithm;
use super::model::artifact::Artifact;

use super::Hash;
use std::fs;
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use std::io::BufReader;


extern crate ctor;

use ctor::*;

pub static ART_MGR: ArtifactManager = None;

//initialize a global instance for artifact manager at startup
#[ctor]
fn init_artifact_manager() {

    let dir_name = format!(
        "{}{}",
        "/var/lib/pyrsia",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            );
        println!("dir for artifacts: {}", dir_name);

        fs::create_dir(dir_name.clone())
            .unwrap_or_else(|e| panic!("Error creating dir for artifacts: {}", e));
        let result =
            ArtifactManager::new(dir_name.as_str());
            ART_MGR = match result {
                Ok(art_mgr) => {
                    println!("Artifact manager created with repo directory {}",
                    ART_MGR.repository_path.display())
                }
                Err(error) => {
                    println!("Error creating artifact manager: {}", error);
                }
        }

}


//get_artifact: given artifact_hash(artifactName) pulls artifact for  artifact_manager and
// returns read object to read the bytes of artifact
pub fn get_artifact(artifact_hash: Vec<u8>) -> Result<File, anyhow::Error> {
    let hash = Hash::new(HashAlgorithm::SHA256, &artifact_hash)?;

    let mut reader = ART_MGR.pull_artifact(&hash)
            .context("Error from get_artifact")?;
            Ok(reader)

}

//put_artifact: given artifact_hash(artifactName) & artifact_path push artifact to artifact_manager
//and returns the boolean as true or false if it was able to create or not
pub fn put_artifact(artifact_hash: Vec<u8>, artifact_path: &str) -> Result<bool, anyhow::Error> {
    let hash = Hash::new(HashAlgorithm::SHA256, &artifact_hash)?;
    let file = File::open(artifact_path).with_context(|| format!("{} not found.", artifact_path))?;
    let mut buf_reader = BufReader::new(file);

    let result = ART_MGR.push_artifact(&mut buf_reader,&hash)
    .context("Error from put_artifact")?;
    Ok(result)

}

/*returns metadata of an artifact_hash i.e. blobs etc
pub fn get_artifact_metadata(art_hash: Vec<u8>) -> Result<Artifact, anyhow::Error> {

    

}*/


