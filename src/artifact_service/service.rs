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

use super::hashing::{Hash, HashAlgorithm};
use super::storage::ArtifactStorage;
use crate::network::client::{ArtifactType, Client};
use crate::transparency_log::log::{TransparencyLog, TransparencyLogError};
use anyhow::{bail, Context};
use libp2p::PeerId;
use log::info;
use multihash::Hasher;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::str;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, strum_macros::Display)]
pub enum PackageType {
    Docker,
    Maven2,
}

pub struct ArtifactService {
    pub artifact_storage: ArtifactStorage,
    pub transparency_log: TransparencyLog,
    pub p2p_client: Client,
}

impl ArtifactService {
    pub fn new<P: AsRef<Path>>(artifact_path: P, p2p_client: Client) -> anyhow::Result<Self> {
        let artifact_storage = ArtifactStorage::new(&artifact_path)?;
        let transparency_log = TransparencyLog::new(&artifact_path)?;
        Ok(ArtifactService {
            artifact_storage,
            transparency_log,
            p2p_client,
        })
    }

    pub fn request_build(&self, _package_type: PackageType, _package_type_id: &str) {}

    pub async fn get_artifact(
        &mut self,
        package_type: PackageType,
        package_type_id: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let transaction = self
            .transparency_log
            .get_artifact(&package_type, package_type_id)?;

        let blob_content = match self.get_artifact_locally(&transaction.hash) {
            Ok(blob_content) => Ok(blob_content),
            Err(_) => self.get_artifact_from_peers(&transaction.hash).await,
        }?;

        self.verify_artifact(&package_type, package_type_id, &blob_content)
            .await?;

        Ok(blob_content)
    }

    pub fn get_artifact_locally(&mut self, artifact_id: &str) -> Result<Vec<u8>, anyhow::Error> {
        let decoded_hash = hex::decode(artifact_id)?;
        let hash: Hash = Hash::new(HashAlgorithm::SHA256, &decoded_hash)?;
        let result = self.artifact_storage.pull_artifact(&hash)?;
        let mut buf_reader: BufReader<File> = BufReader::new(result);
        let mut blob_content = Vec::new();
        buf_reader.read_to_end(&mut blob_content)?;
        Ok(blob_content)
    }

    async fn get_artifact_from_peers(
        &mut self,
        artifact_id: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let providers = self
            .p2p_client
            .list_providers(ArtifactType::Artifact, artifact_id.into())
            .await?;

        match self.p2p_client.get_idle_peer(providers).await? {
            Some(peer) => self.get_artifact_from_peer(&peer, artifact_id).await,
            None => {
                bail!(
                    "Artifact with id {} is not available on the p2p network.",
                    artifact_id
                )
            }
        }
    }

    async fn get_artifact_from_peer(
        &mut self,
        peer_id: &PeerId,
        artifact_id: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let artifact = self
            .p2p_client
            .request_artifact(peer_id, ArtifactType::Artifact, artifact_id.into())
            .await?;

        let decoded_hash = hex::decode(artifact_id)?;
        let hash: Hash = Hash::new(HashAlgorithm::SHA256, &decoded_hash)?;
        let cursor = Box::new(std::io::Cursor::new(artifact));
        self.put_artifact(&hash, cursor)?;
        self.get_artifact_locally(artifact_id)
    }

    async fn verify_artifact(
        &mut self,
        package_type: &PackageType,
        package_type_id: &str,
        blob_content: &[u8],
    ) -> Result<(), TransparencyLogError> {
        let mut sha256 = multihash::Sha2_256::default();
        sha256.update(blob_content);
        let calculated_hash = hex::encode(sha256.finalize());

        let transaction = self
            .transparency_log
            .get_artifact(package_type, package_type_id)?;
        if transaction.hash == calculated_hash {
            Ok(())
        } else {
            Err(TransparencyLogError::InvalidHash {
                id: package_type_id.to_string(),
                invalid_hash: calculated_hash,
                actual_hash: transaction.hash,
            })
        }
    }

    //put_artifact: given artifact_hash(artifactName) & artifact_path push artifact to artifact_manager
    //              and returns the boolean as true or false if it was able to create or not
    fn put_artifact(
        &self,
        artifact_hash: &Hash,
        art_reader: Box<dyn Read>,
    ) -> Result<(), anyhow::Error> {
        info!("put_artifact hash: {}", artifact_hash);
        let mut buf_reader = BufReader::new(art_reader);
        self.artifact_storage
            .push_artifact(&mut buf_reader, artifact_hash)
            .context("Error from put_artifact")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::client::command::Command;
    use crate::network::idle_metric_protocol::PeerMetrics;
    use crate::transparency_log::log::{AddArtifactRequest, TransparencyLogError};
    use crate::util::test_util;
    use anyhow::Context;
    use libp2p::identity::Keypair;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;
    use std::env;
    use std::fs::File;
    use std::path::PathBuf;
    use tokio::sync::{mpsc, oneshot};
    use tokio::task;

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];

    #[tokio::test]
    async fn test_put_and_get_artifact() {
        let tmp_dir = test_util::tests::setup();

        let (add_artifact_sender, _receiver) = oneshot::channel();
        let (sender, _receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let package_type = PackageType::Docker;
        let artifact_id = "an_artifact_id";
        artifact_service
            .transparency_log
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_type_id: artifact_id.to_string(),
                    hash: hex::encode(VALID_ARTIFACT_HASH),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let hash = Hash::new(HashAlgorithm::SHA256, &VALID_ARTIFACT_HASH).unwrap();
        //put the artifact
        artifact_service
            .put_artifact(&hash, Box::new(get_file_reader().unwrap()))
            .context("Error from put_artifact")
            .unwrap();

        // pull artifact
        let future = {
            artifact_service
                .get_artifact(package_type, artifact_id)
                .await
                .context("Error from get_artifact")
        };
        let file = task::spawn_blocking(|| future).await.unwrap().unwrap();

        //validate pulled artifact with the actual data
        let mut s = String::new();
        get_file_reader().unwrap().read_to_string(&mut s).unwrap();

        let s1 = match str::from_utf8(file.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(s, s1);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_from_peers() {
        let tmp_dir = test_util::tests::setup();

        let (sender, mut receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Some(Command::ListProviders { artifact_type: _artifact_type, artifact_hash: _artifact_hash, sender }) => {
                        let mut set = HashSet::new();
                        set.insert(local_peer_id);
                        let _ = sender.send(set);
                    },
                    Some(Command::RequestIdleMetric { peer: _peer, sender }) => {
                        let _ = sender.send(Ok(PeerMetrics {
                            idle_metric: (0.1_f64).to_le_bytes()
                        }));
                    },
                    Some(Command::RequestArtifact { artifact_type: _artifact_type, artifact_hash: _artifact_hash, peer: _peer, sender }) => {
                        let _ = sender.send(Ok(b"SAMPLE_DATA".to_vec()));
                    },
                    _ => panic!("Command must match Command::ListProviders, Command::RequestIdleMetric, Command::RequestArtifact"),
                }
            }
        });

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let hash_bytes = hasher.finalize();
        let artifact_id = hex::encode(hash_bytes);

        let future = { artifact_service.get_artifact_from_peers(&artifact_id).await };
        let result = task::spawn_blocking(|| future).await.unwrap();
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_from_peers_with_no_providers() {
        let tmp_dir = test_util::tests::setup();

        let (sender, mut receiver) = mpsc::channel(1);
        let peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id: peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        tokio::spawn(async move {
            tokio::select! {
                command = receiver.recv() => {
                    match command {
                        Some(Command::ListProviders { artifact_type: _artifact_type, artifact_hash: _artifact_hash, sender }) => {
                            let _ = sender.send(Default::default());
                        },
                        _ => panic!("Command must match Command::ListProviders"),
                    }
                }
            }
        });

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let hash_bytes = hasher.finalize();
        let artifact_id = hex::encode(hash_bytes);

        let future = { artifact_service.get_artifact_from_peers(&artifact_id).await };
        let result = task::spawn_blocking(|| future).await.unwrap();
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_artifact_succeeds_when_hashes_same() {
        let tmp_dir = test_util::tests::setup();

        let (add_artifact_sender, _receiver) = oneshot::channel();
        let (sender, _receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let package_type = PackageType::Docker;
        let package_type_id = "package_type_id";
        artifact_service
            .transparency_log
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_type_id: package_type_id.to_string(),
                    hash: random_hash,
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let result = artifact_service
            .verify_artifact(&package_type, package_type_id, b"SAMPLE_DATA")
            .await;
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_artifact_fails_when_hashes_differ() {
        let tmp_dir = test_util::tests::setup();

        let (add_artifact_sender, _receiver) = oneshot::channel();
        let (sender, _receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(b"OTHER_SAMPLE_DATA");
        let random_other_hash = hex::encode(hasher2.finalize());

        let package_type = PackageType::Docker;
        let package_type_id = "package_type_id";
        artifact_service
            .transparency_log
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_type_id: package_type_id.to_string(),
                    hash: random_hash.clone(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let result = artifact_service
            .verify_artifact(&package_type, package_type_id, b"OTHER_SAMPLE_DATA")
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            TransparencyLogError::InvalidHash {
                id: package_type_id.to_string(),
                invalid_hash: random_other_hash,
                actual_hash: random_hash
            }
        );

        test_util::tests::teardown(tmp_dir);
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }
}
